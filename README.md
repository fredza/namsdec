# namsdec

**Un atelier pour apprendre l'assembleur NASM x86-64 — et pour lire les
binaires des autres.**

namsdec exécute votre programme instruction par instruction et vous dit,
à chaque pas, ce que la machine est sur le point de faire et ce qu'elle
vient de faire. Il ne se contente pas d'afficher des registres : il
explique. C'est un outil pédagogique, sous licence MIT, fait pour que ce
qu'on y comprend puisse se transmettre.

> Interface en français et en anglais. Linux, ELF64, syntaxe NASM.

---

## Ce qu'il montre que les autres ne montrent pas

**L'instruction suivante, résolue.** `mov al, [rsi + rcx*4 + 8]` ne dit
pas *où* ça pointe : il faut tenir trois registres de tête et faire le
calcul. namsdec le fait avant l'exécution — l'adresse effective, son
étiquette dans votre source (`tampon+2`), et l'octet qui s'y trouve.

**Le dernier pas, avec la valeur d'avant.** `RCX 0x5 → 0x4` sous le
`dec rcx` qui l'a fait, et les indicateurs modifiés à côté. Après un
`cmp`, ce sont eux le résultat : l'instruction ne touche aucun registre.

**Le saut conditionnel, avant qu'il parte.** « Le saut sera pris »,
la règle qu'il suit (`CF = 0 et ZF = 0`), et si la comparaison lit ses
nombres comme signés. `ja` et `jg` disent tous les deux « plus grand » et
répondent l'inverse sur les mêmes octets : c'est exactement là qu'on se
trompe.

**L'appel système écrit en clair.** `syscall` ne porte aucun opérande —
tout est dans les registres, selon une convention qu'on connaît ou qu'on
va chercher ailleurs :

```
write(fd = 0x1, buf = 0x402000 → "salut\n", count = 0x6)
```

**La pile lue comme une pile.** Un mot de huit octets par ligne, RSP et
RBP repérés, et toute valeur qui pointe dans le code nommée comme ce
qu'elle est : une adresse de retour, avec son étiquette. Ce qui est
au-dessus de RSP est grisé — ces octets sont encore là, mais le prochain
`push` les écrase.

**Ce qui ressort, et pourquoi.** La fenêtre d'analyse ouvre sur
**Signaux** : segment inscriptible *et* exécutable, entropie qui monte,
empaqueteur reconnu par son nom, anti-débogage, protections absentes,
indicateurs dans les chaînes. Rien de neuf n'y est calculé — c'est ce que
les autres panneaux ont déjà trouvé, rassemblé pour dire par où
commencer. Chaque ligne donne un **fait** (une adresse, un nom, une
mesure), **ce que ce fait change**, et **le panneau** qui l'explique. Rien
n'est additionné en score : un score donnerait l'illusion d'un verdict, et
c'est le lecteur qui conclut.

**Ce que le binaire sait faire, en six familles.** Les imports et les
appels système sont listés partout ; ici, ils sont aussi *rangés* —
réseau, exécution, anti-débogage, mémoire, chiffrement, fichiers. La
question n'est pas « quelles fonctions » mais « qu'est-ce que ce binaire
fait », et trois familles allumées sont déjà un portrait. Un `socket` est
un `socket`, qu'il arrive par la table de liaison ou par le numéro 41
dans RAX — et un programme qui veut passer inaperçu emploie justement le
second.

**Et namsdec peut aller le chercher.** Rizin + rz-ghidra se compile,
RetDec se télécharge : Préférences ▸ Modules ▸ « Aller chercher les outils
et les construire », puis **Installer…**. Ce module est éteint par défaut,
et plus franchement que les autres — les autres affichent, celui-ci
télécharge du code et l'exécute. La fenêtre montre donc **la recette
entière avant que rien ne parte**, et le journal s'écrit en direct.
Aucune étape n'appelle `sudo`, aucune ne passe par un interpréteur de
commandes, et rien n'est posé hors de
`~/.local/share/namsdec/tools/`. Le binaire n'en grossit pas :
`curl` télécharge, `tar` extrait, `meson` et `cmake` construisent — comme
`nasm` et `ld` assemblent déjà.

**Un décompilateur, si vous en avez un.** Le module **Décompilation**
(Préférences ▸ Modules, éteint par défaut) lance l'outil de votre choix.
Six sont connus d'avance — `objdump`, `ndisasm`, le pseudo-code de
`radare2`, **Rizin + rz-ghidra**, **RetDec** et **DZF** — avec les
arguments qui marchent.

Chaque fiche répond à **deux questions**, et il faut les deux pour que
l'outil serve : le programme est-il **installé sur la machine**, et est-il
**activé dans namsdec** ? Les deux sont indépendantes — un outil installé
mais non activé ne sera pas lancé, et un outil activé qui a disparu de la
machine échouera au lancement, ce que la fiche dit en rouge. L'activer
**écrit un fichier de quelques lignes** : namsdec ne télécharge rien et ne
lance rien de lui-même.

La fiche d'un outil absent dit **comment l'obtenir sans rien déplier** :
c'est une panne, pas un détail. Le repli « Détails » ne garde que la
commande exacte qui sera lancée. Les fiches s'ouvraient d'office quand
l'outil manquait, et sur une machine neuve — où aucun ne l'est — les deux
derniers outils passaient sous le bord de la fenêtre.

Un outil qui répond ne propose que **Fermer** : relancer vingt minutes de
compilation pour un résultat identique n'est pas une offre. Un outil
**incomplet** propose **Réparer** — Rizin sans `rz-ghidra`, où `pdg`
n'existe pas et où rien ne disait quoi faire.

**DZF est dans la liste bien qu'il soit en cours d'écriture** : c'est le
désassembleur x86-64 maison, en C++23 et sans aucune dépendance. Il ne lit
pas l'ELF et n'a pas à l'apprendre — namsdec lui passe les octets
exécutables et leur adresse. Tant qu'il n'est pas bâti, sa fiche dit où le
prendre. Le vôtre s'ajoute de la
même façon, par un JSON déposé dans
`~/.local/share/namsdec/decompilers/` — avec `{out}` dans les arguments
si l'outil écrit son résultat dans un fichier plutôt que sur la sortie
standard. Trois
verrous, parce qu'il s'agit d'exécuter un programme sur votre machine :
le module s'active à la main, la fenêtre s'ouvre à la demande, et
**la commande est écrite en toutes lettres avant d'être lancée**. Le
travail se fait à côté — l'interface ne se fige pas —, et un outil qui ne
rend pas la main au bout de deux minutes est arrêté plutôt qu'attendu
sans fin. L'avancement se lit dans la barre d'état **et dans le titre de
la fenêtre**, donc dans la barre des tâches du système : « Namsdec —
Décompilation 45 % ».

**Un panneau de désassemblage, avec le choix du moteur.** Le pseudo-code
d'abord — `RAX = RAX + RCX` plutôt que `add rax, rcx`, `si différent,
aller à boucle` plutôt que `jne` —, la syntaxe NASM ensuite, et les
outils qu'on installe soi-même : un fichier JSON dans
`~/.local/share/namsdec/disassemblers/` décrit la commande à lancer
(`objdump`, `ndisasm`, le vôtre). Ceux-là sont des **programmes** : ils
ne s'exécutent que sur demande, et le panneau annonce ce qu'il va lancer
avant de le faire.

**D'où vient cette valeur.** Survoler un registre dit quel pas y a écrit
en dernier, son rang et sa ligne : « écrit au pas -12, ligne 7 :
add rax, rcx ». Le chercher à la main veut dire relire son programme à
l'envers, et se tromper une fois sur deux dès qu'il y a une boucle.

**Plusieurs fichiers ouverts à la fois.** Une barre d'onglets au-dessus
de l'éditeur, le nom raccourci par le milieu quand il est long — pour que
`boucle_i…ee.asm` et `boucle_i…e_2.asm` restent deux fichiers
différents —, un point sur ce qui n'est pas enregistré. Un exercice
ouvert prend aussi son onglet, sous son titre. Chaque onglet
garde **tout** ce qu'on y faisait : le texte tapé, le curseur, les points
d'arrêt, et l'exécution arrêtée au pas où on l'avait laissée. Ctrl+Tab
passe au suivant, Ctrl+W ferme, et les fichiers reviennent au lancement
d'après.

**La forme du programme, pas sa suite d'instructions.** `Espace g` ouvre
le **déroulement** : chaque boîte est un bloc qu'on ne peut qu'entrer par
le haut et quitter par le bas, les flèches disent le saut pris (vert), le
saut non pris (rouge) et l'appel (bleu), et une boucle se voit à ce
qu'elle remonte. Le bloc où le programme se trouve est mis en avant, un
clic y emmène dans l'éditeur, et Ctrl+molette recule pour voir la forme
d'ensemble. C'est ce qu'on reconstitue de tête en lisant un listing — et
ce qu'un débutant ne voit pas encore.

**Où le programme passe son temps.** Une bande de chaleur dans la
gouttière — verte pour une ligne parcourue une fois, rouge pour la plus
chaude, jaune et orange entre les deux — avec le compte exact au survol
et une légende sous l'éditeur. L'onglet **Profil** de la console en fait
un classement : le nombre de passages, la part du total, la ligne
elle-même, et un clic qui y emmène. La bande dit où ça chauffe ; le
classement dit de combien, et se copie en texte.

**Ce que namsdec ne fera pas.** `mmap`, `open`, `fork` : ces appels
système sont nommés mais pas exécutés — namsdec n'a ni fichiers ni
processus à offrir. Le programme qui en contient un s'arrêterait dessus
sans prévenir : c'est dit à l'assemblage, avec la ligne.

**Ce que nasm ne dit pas.** Deux fautes passent l'assemblage sans un mot
et ne se voient qu'à l'exécution — ou pas du tout : un programme que rien
n'arrête, qui continue dans ce qui suit son code, et une écriture à une
adresse fixe qui tombe dans une zone en lecture seule. namsdec relit le
programme lié et les signale comme des avertissements, avec leur ligne.

**Ce qu'il faut faire quand ça plante.** Le message dit « interdit
d'écrire à 0x400000 : elle est dans .text ». Dessous : *le code et les
constantes sont en lecture seule, ce qui change pendant l'exécution va
dans `section .data`*.

**Qui appelle quoi.** Les références croisées relèvent, pour chaque
adresse, qui l'appelle, qui y saute et qui la désigne. Une adresse qu'un
`call` vise est une fonction, même dépouillée de son nom — et celle qui
est appelée de partout est celle par laquelle on commence à lire.

**Comment arrive-t-on là ?** Dans le déroulement, un champ prend une
adresse — celle où le programme accepte, par exemple — et donne **la route
pour y parvenir**, avec les conditions à satisfaire dans l'ordre : ce
`jne` ne doit pas être pris, celui-là doit l'être. Les blocs du chemin
sont mis en avant dans le dessin. C'est la route la plus courte, et
namsdec dit quand il en existe d'autres plutôt que de laisser croire à une
démonstration.

**Ce que le code fait de remarquable.** Le module **Figures** relève les
constantes qui sont du texte (`mov ecx, 0x48555648` → `"HVUH"`), les
contrôles de longueur, les comparaisons déguisées en XOR — `a XOR b` ne
vaut zéro que si les deux sont égaux, donc la constante *est* la valeur
attendue —, les comparaisons de chaîne et les tables de sauts. Les
familles sont rangées **de la plus rare à la plus commune** : dans un
binaire de soixante mille instructions, mille sept cents tests de nullité
sont de la ponctuation, et quatre comparaisons par XOR sont une piste.

**Par où commencer à lire.** Le module **Fonctions** retrouve le `main`
d'un binaire dépouillé — le code d'amorçage passe son adresse à
`__libc_start_main`, dans RDI — puis découpe le code sur les cibles
d'appel, avec le nombre d'appelants de chacune. Et les chaînes des
langages sans zéro final (Rust, Go, Zig) sont **découpées par le code**,
là où un scan des octets ne lit qu'un pâté : l'adresse y voyage avec sa
longueur.

**D'où vient cette valeur.** Survoler un registre dit quel pas y a écrit
en dernier. Quand il n'y a pas d'historique — un binaire qui appelle une
bibliothèque absente ne s'exécute pas —, namsdec remonte le **code** à
l'envers jusqu'à la source, en s'arrêtant au premier saut : au-delà,
plusieurs chemins mènent là, et « la dernière écriture » n'a plus de sens
unique.

**Les protections d'un binaire.** PIE, NX, RELRO, canari de pile,
FORTIFY, IBT et pile fantôme, segments W+X, symboles, bibliothèques,
RPATH, et les fonctions qu'il emprunte — chaque entrée de la table de
liaison retrouve son nom, avec le nombre d'appels qui y mènent — avec,
pour chacun, ce que son état change concrètement. Sur
votre programme comme sur un binaire du système. Le rapport se copie en
Markdown, ou s'enregistre, pour être collé dans un compte rendu ou un cours.

Et aussi : documentation de 164 instructions, directives et notions sous **F1** ou
au survol — avec sommaire, renvois d'une fiche à l'autre et export du
manuel entier en Markdown (menu Aide), parce que ce qu'on apprend ici
doit pouvoir se lire ailleurs —, autocomplétion qui ne propose que ce qui est documenté,
motif cyclique de De Bruijn pour savoir de combien un débordement
dépasse (identique à celui de pwntools, donc comparable),
palette de commandes qui mène aussi aux étiquettes du programme (on tape leur nom, on y est),
une chronologie où l'on clique un pas passé pour y revenir (registres, indicateurs et mémoire avec), registres et indicateurs qui se posent à la main (« et si ZF valait
1 ? »), points d'arrêt — avec condition (`rcx == 0`), surveillance d'un registre
ou d'une adresse, vérification de la convention d'appel (ce qu'une
sous-routine a emprunté sans le rendre), mémoire protégée par les droits
réels des sections, tas simulé avec `brk`, SSE/AVX et pile x87.

## Les plugins

Le dossier [`plugins/`](plugins/) range ce qui s'ajoute à namsdec sans le
recompiler, en deux familles :

- [**Thèmes**](plugins/themes/) — des couleurs, et rien d'autre. Un thème
  ne s'exécute pas.
- [**Fonctionnalités supplémentaires**](plugins/fonctionnalites/) — la
  description d'un **programme** à lancer : `objdump`, `ndisasm`,
  `radare2`, Rizin + rz-ghidra, RetDec. Chacun avec ses instructions
  d'installation, et les arguments qui marchent.

La distinction n'est pas un classement, c'est une question de confiance :
namsdec ne va jamais chercher un programme tout seul, ne le lance que sur
demande explicite, et écrit la commande complète avant de la lancer.

## Le guide

Un **guide de l'utilisateur en PDF** accompagne chaque version :
[`docs/guide-namsdec-0.2.6-beta.1.pdf`](docs/guide-namsdec-0.2.6-beta.1.pdf)
— installation, prise en main, chaque panneau, l'exécution pas à pas, la
mémoire, le manuel, le déroulement, le parcours, la rétro-ingénierie, les
raccourcis et les limites connues.

Il est **produit depuis le dépôt** (`python3 docs/guide.py`) : la version,
le nombre d'exercices, le nombre de fiches et la table des raccourcis
viennent des sources, jamais d'une copie qui vieillirait.

## Installer

Il faut **Rust** (`cargo`), **nasm** et **ld** (binutils) :

```sh
# Fedora
sudo dnf install nasm binutils
# Debian, Ubuntu
sudo apt install nasm binutils
```

Puis :

```sh
git clone https://github.com/fredza/namsdec.git
cd namsdec
./install.sh
```

`install.sh` compile, pose le binaire dans `~/.local/bin`, l'icône et le
lanceur là où le bureau va les chercher. Rien n'est écrit hors de `$HOME`
et rien ne demande les droits d'administrateur.

`./install.sh --help` donne les options. Les plus utiles :

| option | ce qu'elle fait |
|---|---|
| `--dry-run`, `-n` | dire ce qui serait fait, sans rien écrire |
| `--debug` | installer la version non optimisée, prête en quelques secondes |
| `--prefix DIR` | où vont icônes, lanceur et exercices |
| `--bin-dir DIR` | où va le binaire |
| `--no-desktop` | pas de lanceur ni d'icônes |
| `--no-dock` | ne pas épingler au dock (GNOME) |
| `--no-path` | ne pas toucher au fichier de configuration du shell |
| `--no-exercises` | ne pas installer la série livrée |
| `--no-strip` | garder les symboles de débogage (deux mégaoctets de plus) |
| `--no-build` | ne pas compiler : poser le binaire déjà construit |
| `--quiet`, `-q` | ne dire que l'essentiel |
| `--no-color` | pas de couleurs — `NO_COLOR` est respecté aussi |

**Un binaire plus léger** : `cargo build --release --no-default-features`
retire la recherche de mise à jour, et avec elle la pile TLS —
**16,0 → 14,4 Mo** une fois dépouillé, soit dix pour cent. C'est ce que
veut un empaqueteur de distribution : les mises à jour sont son travail,
pas celui de l'application. Le menu n'en montre alors pas l'entrée,
plutôt que d'offrir un bouton qui ne peut rien faire.

`./uninstall.sh` défait exactement cela, et prend `--prefix`,
`--bin-dir`, `--dry-run`, `--quiet` et `--no-color` de la même façon. `--purge` retire en plus vos réglages
et vos séries d'exercices — sans lui, ils sont gardés, parce qu'on
désinstalle souvent pour réinstaller.

Pour lancer sans installer : `cargo run -p namsdec-gui`.

## Prendre en main

1. **Ctrl+Maj+B** assemble le programme (nasm puis ld, pour de vrai).
2. **Espace p i** avance d'une instruction, **Espace p o** passe par-dessus
   un `call`, **Espace p r** lance, **Espace p b** revient en arrière d'un
   pas. Les séquences sont rangées par famille : `p` pour ce qui fait
   tourner le **programme**, `/` pour chercher, `m` pour **marquer**, `v`
   pour les **vues**, `u` pour courir **jusqu'à** quelque chose.
3. **F1** sur un mot du code en donne la fiche ; le survol aussi.
4. Un clic dans la gouttière pose un point d'arrêt ; **Ctrl+F10** exécute
   jusqu'à la ligne du curseur.
5. **Ctrl+Maj+P** ouvre la palette : toutes les commandes s'y trouvent.

Les raccourcis sont dans Préférences ▸ Raccourcis, et se changent.

### Les exercices

Quatre-vingt-neuf des quatre-vingt-seize exercices portent un **but vérifiable** (`rax == 0xf`) :
le bandeau de l'état du processeur dit « atteint » ou « pas encore », au
lieu de laisser vérifier à l'œil. Ce qui a été réussi une fois le reste :
une pastille dans la liste, un compte en tête de la série, et les deux
survivent au lancement suivant.

Chaque exercice mesuré porte aussi un **budget** : le nombre
d'instructions qu'exécute la solution livrée. L'égaler est normal ;
descendre en dessous est le vrai exercice — c'est en cherchant comment
qu'on apprend `lea`, les décalages, ou qu'une boucle de cinq tours se
remplace par une formule. Le meilleur score est retenu par exercice, un
anneau marque ceux passés sous le budget, et un test vérifie que chaque
budget annoncé est tenable : promettre un chiffre qu'aucune solution
n'atteint serait pire que de n'en pas donner. Une série installée peut en porter
aussi — champ `goal` du fichier JSON.

Un parcours de quatre-vingt-seize exercices est livré avec le
programme, rangé en quatre paliers de difficulté croissante :

- **Découverte** (21) — registres, échange, mise à zéro, boucles,
  `loop`, bits, comparaisons, indicateurs, signe, nombres négatifs,
  écritures d'un nombre, `inc` contre `add`, `test`, `xchg`/`bswap`,
  masques de bits, constantes calculées à l'assemblage, rotations,
  `setcc`, `not` et `neg`, division signée, coût d'un immédiat.
- **Pratique** (26) — décalages, compte de bits, multiplication,
  division, mémoire, tailles d'opérande, ordre des octets, données
  déclarées, tableaux, matrices, recherche, calcul d'adresse, minimum et
  maximum, parité, occurrences, retournement sur place, bornage sans
  branchement, puissances de deux, somme et moyenne, tableau d'octets,
  zones qui se chevauchent (`memmove`), remplissage par `rep stosb`,
  recherche de bit, addition sur 128 bits, table de conversion,
  alignement et trous d'une structure.
- **Structures** (30) — chaînes, instructions de chaîne, comparaison de
  chaînes, texte vers nombre, macros, enregistrements et champs, table
  de saut, choix sans branchement, pile, appels, convention d'appel,
  récursion, Fibonacci, cadre de pile, tableau local, tri, `strlen` par
  `scasb`, mot à l'envers, compte de mots, pile qui retourne, arguments
  au-delà de six, pointeur de fonction, alignement de pile, concaténation, extraction
  d'un mot, récursion terminale, tri par insertion, recherche binaire,
  liste chaînée, transposition.
- **Maîtrise** (19) — sortie, entrée, tas, flottants, SIMD, affichage
  décimal et hexadécimal, majuscules, débordement, protections,
  chiffrement XOR, somme de contrôle, canari posé à la main, choix
  vectoriel par masque, somme vectorielle, troncature contre arrondi,
  table ASCII, affichage d'un négatif, lecture jusqu'à la fin.

Activable dans Préférences ▸ Modules.

**Le parcours n'est pas dans le binaire** : c'est un fichier de données,
`~/.local/share/namsdec/exercises/namsdec.json`, installé par
`install.sh` et lu au démarrage comme n'importe quelle autre série.
Corriger un énoncé ne demande donc pas de recompiler, ce qu'on écrit
soi-même se range exactement comme ce qui est livré, et le programme ne
porte pas cent kilo-octets de texte. D'autres séries s'installent de la
même façon : Outils ▸ Exercices.

#### L'Atelier, seconde série

Dix-huit exercices de plus, livrés à part dans
`plugins/exercices/atelier.json` et installés par le même `./install.sh` :
ils reprennent là où le parcours s'arrête.

- **Structures** (8) — une matrice et la multiplication d'indice qui la
  fait exister, la longueur d'une chaîne, un tri par insertion, la
  récursion, un cadre de pile monté à la main, une liste chaînée, la
  recherche d'un octet, une table de sauts.
- **Maîtrise** (10) — quatre additions en une instruction (`paddd`),
  seize octets comparés d'un coup (`pcmpeqb` puis `pmovmskb`), un appel
  système qui écrit vraiment, le débordement signé et le non signé qui
  ne répondent pas à la même question, un produit qui ne tient pas dans
  un registre, diviser par décalage et le piège du signe, l'ordre des
  octets, le compte de bits allumés, un canari de pile, un programme
  qui voudrait se récrire.

Ces derniers ne sont pas des curiosités : ce sont les formes qu'on
rencontre en lisant un binaire réel. Un `bswap` trahit du réseau ou un
format de fichier ; un `mov rax, fs:[0x28]` en tête de fonction trahit
la protection de pile. Les reconnaître est la moitié du travail
d'analyse.

Les dix-huit s'assemblent et **atteignent leur but tels qu'ils sont
livrés** — un test l'exécute à chaque fois, parce qu'un exercice dont le
code donné manque son propre but apprend la mauvaise chose.

### Si vous venez de pwndbg

namsdec n'est pas un débogueur : il simule le processeur au lieu d'en
piloter un. Mais les gestes qu'on fait devant un binaire sont les mêmes,
et ils portent ici les mêmes noms qu'ailleurs.

| pwndbg / gdb | namsdec |
| --- | --- |
| `telescope` | onglet Mémoire, lecture « Pointeurs » ; et chaque mot de la pile |
| `vmmap` | onglet Mémoire ▸ Carte de la mémoire |
| `search` | onglet Mémoire ▸ Chercher en mémoire |
| `patch` | onglet Mémoire ▸ Écrire ici |
| `set $rax = …` | clic sur la valeur du registre |
| `setflag ZF 1` | clic sur l'indicateur |
| `cyclic` | Outils ▸ Motif cyclique (identique à pwntools) |
| `dumpargs` | bandeau, devant un `call` |
| `retaddr` | onglet Pile ▸ Adresses de retour |
| `nextcall`, `nextret`, `nextsyscall` | Outils ▸ Jusqu'au prochain… (`Espace u c/r/s`) |
| `context` | Édition ▸ Copier l'état |
| `checksec` | fenêtre Analyse ▸ Protections |
| (ce qu'on regarde en ouvrant un échantillon) | fenêtre Analyse ▸ Signaux |
| `got`, `plt` | fenêtre Analyse ▸ Fonctions importées |
| `nearpc` | le listing lui-même, RIP dans la gouttière |
| `stepi`, `nexti`, `reverse-stepi` | `Espace p i`, `Espace p o`, `Espace p b` |

### Trois façons de lire la même instruction

La fenêtre de désassemblage offre un choix de moteur :

| moteur | ce qu'il montre | exemple |
|---|---|---|
| Pseudo-code | l'instruction dite en français | `RAX = RBX` |
| NASM | la syntaxe qu'on écrit dans l'éditeur | `mov rax, rbx` |
| Système | celle de `gdb`, `objdump` et `as` | `mov %rbx, %rax` |

La syntaxe **Système** est une vue, non une écriture : nasm ne la lit
pas. Elle est là parce que tout ce qu'on trouve hors de namsdec est
écrit dans celle-là.

### Ajouter un désassembleur

Le panneau de désassemblage propose deux moteurs livrés — le pseudo-code
et la syntaxe NASM — et tous ceux qu'on lui décrit. Un fichier JSON par
outil, dans `~/.local/share/namsdec/disassemblers/` :

```json
{
  "id": "objdump",
  "name": "objdump",
  "command": "objdump",
  "args": ["-d", "-M", "intel", "{file}"]
}
```

`{file}` est remplacé par le chemin du binaire. Contrairement à un thème,
qui n'est qu'une palette, **un tel plugin est un programme** : namsdec ne
le lance qu'au clic, annonce la commande avant de le faire, passe le
binaire par un fichier temporaire plutôt que par un interpréteur de
commandes, et borne la sortie. Un fichier qui ne peut pas marcher —
commande vide, ni `{file}` ni `{text}` — est refusé à la lecture plutôt
que découvert au premier essai.

**Si l'outil ne lit pas l'ELF**, deux autres marqueurs le dispensent de
l'apprendre :

| marqueur | ce que namsdec y met |
|---|---|
| `{file}` | le binaire entier, tel qu'il est sur le disque |
| `{text}` | les octets **exécutables seuls**, sans en-tête ni données |
| `{base}` | l'adresse où ces octets sont chargés, en hexadécimal |
| `{out}` | un fichier où écrire, pour les outils qui ne parlent pas sur la sortie standard |

Sans eux, un désassembleur qui décode des octets bruts commence par
désassembler l'en-tête ELF comme du code — les vingt premières lignes ne
veulent rien dire — et adresse tout son listing à partir de zéro, si bien
qu'aucun saut ne tombe juste. C'est namsdec qui sait lire l'ELF ; il n'y
a pas de raison de le faire réapprendre à chaque outil.

C'est le cas de **DZF** (`plugins/fonctionnalites/dzf.json`), le
désassembleur x86-64 maison — C++23, écrit à la main, sans dépendance :

```json
{
  "id": "dzf",
  "name": "DZF — désassemblage x86-64 (maison)",
  "command": "x64-disassembler",
  "args": ["--base", "{base}", "{text}"]
}
```

Il n'a rien eu à changer pour cela : c'est la fiche qui dit ce qu'il
veut recevoir.

## Le code

Un workspace Rust, une crate par responsabilité :

| crate | ce qu'elle fait |
|---|---|
| `namsdec-core` | registres, indicateurs, mémoire et ses droits |
| `namsdec-asm` | appel de nasm et de ld, lecture de l'ELF produit |
| `namsdec-disasm` | désassemblage x86-64 (iced-x86), syntaxe NASM |
| `namsdec-emu` | l'interpréteur : entiers, SSE/AVX, x87, appels système |
| `namsdec-re` | rétro-ingénierie : signaux, protections, références croisées, chaînes et indicateurs, familles de comportement, appels système, empaqueteurs, fragments ROP, place libre |
| `namsdec-doc` | les fiches d'instructions et de directives |
| `namsdec-exercises` | les séries d'exercices et leur format |
| `namsdec-theme` | les thèmes, y compris ceux installés depuis le disque |
| `namsdec-explorer` | l'explorateur de fichiers |
| `namsdec-update` | la recherche de version publiée, et la comparaison SemVer |
| `namsdec-gui` | l'interface (egui / eframe) |

Les crates métier ne connaissent pas la langue de l'interface : leurs
erreurs portent un message anglais et gardent leurs valeurs dans des
variantes structurées, que l'interface traduit.

```sh
cargo test --workspace      # plus de 330 tests
cargo clippy --workspace --all-targets
```

Les tests ne se contentent pas de vérifier des fonctions : ils assemblent
et exécutent de vrais programmes avec nasm et ld, dessinent l'interface
hors écran et lisent ce qui a été peint. Chaque programme livré avec
l'application, et chaque exemple des fiches, doivent s'assembler, se lier
sans avertissement, et s'exécuter jusqu'au bout.

## Ce qu'il ne fait pas

- Pas de mode 32 bits, pas d'`int 0x80`.
- **Pas d'éditeur de liens dynamique.** namsdec charge un binaire comme
  le ferait le noyau — segments projetés en pages, relocations
  appliquées, mémoire locale au fil posée, pile de processus écrite avec
  `argc`, `argv`, l'environnement et le vecteur auxiliaire — mais il ne
  charge aucune bibliothèque. Un binaire lié **statiquement** s'exécute
  donc entièrement, bibliothèque C comprise.
- À la place, namsdec **fournit lui-même** une centaine de fonctions,
  écrites en Rust : `printf` et ses cousines, `puts`, `fgets`, `scanf`,
  les fonctions de chaînes et de mémoire, `malloc` et sa famille — qui
  prend sa mémoire dans le tas, où on la voit —, et les enveloppes des
  appels système. Un binaire **dynamique** ordinaire s'exécute donc lui
  aussi, du prompt jusqu'au verdict ; il s'arrête en **nommant** la
  première fonction qu'il emprunte et que namsdec ne fournit pas. Ce qui
  est fourni l'est en le disant : le compte s'affiche au chargement, et
  la liste complète est celle de `namsdec_emu::PROVIDED_FUNCTIONS`.
- Ces fonctions obéissent aux mêmes règles que le reste : les droits de
  la mémoire simulée s'appliquent (un `strcpy` vers `.rodata` échoue,
  comme sur la vraie machine), chaque appel compte pour **un pas**, et ce
  qu'il écrit se défait avec « pas en arrière ».
- Les appels système simulés sont ceux qui *font* le travail — `read`,
  `write`, `brk`, `mmap`, `mprotect`, `exit` —, ceux qui renseignent sans
  rien changer au monde — `getpid`, `getuid`, `geteuid`, `sched_yield`,
  `nanosleep`, `getrandom`, `uname`, `clock_gettime` — et ceux du
  démarrage d'une bibliothèque C : `arch_prctl`, `set_tid_address`,
  `fstat`, `ioctl`, `writev`, `prlimit64`, `futex`. Les autres — ouvrir
  un fichier, lancer un processus, ouvrir une connexion — sont nommés et
  signalés comme non simulés : il n'y a derrière namsdec ni système de
  fichiers, ni réseau, ni second processus. `getrandom` rend des octets
  **reproductibles** : c'est ce qu'il faut à un atelier où l'on revient
  en arrière, et ce qu'il ne faut surtout pas à une clé.
- AVX-512, FMA, les ports `in`/`out` et `cmpxchg16b` ne sont pas
  implémentés.

### La fenêtre

namsdec dessine sa propre barre de titre : les menus en tiennent lieu,
et les trois pastilles sont **en tête**, à gauche — fermer, réduire,
agrandir. La fermeture occupe ainsi le coin de l'écran, l'angle qu'une
souris lancée sans viser atteint, et les deux autres restent derrière
elle. On déplace la fenêtre en saisissant la barre, on l'agrandit d'un
double-clic, on la redimensionne par ses bords. Préférences ▸ Général rend la décoration du
bureau à qui la préfère — le réglage s'applique sans redémarrer.

### Sortir de l'atelier

Fichier ▸ **Exporter le binaire** écrit le programme tel que `ld` l'a
produit — un ELF exécutable, celui que la ligne de commande aurait
donné : il se lance dans un terminal, se lit avec `objdump`, se compare
avec ce que namsdec en dit. L'onglet Diagnostics affiche les **deux
commandes réellement lancées**, prêtes à copier :

```sh
nasm -f elf64 -o programme.o programme.asm
ld -Ttext 0x400000 -o programme programme.o
```

Un atelier qui ne laisse rien sortir apprend à se servir de lui, pas de
l'assembleur.

### Les mises à jour

« À propos ▸ Rechercher une mise à jour » demande à GitHub les versions
publiées et compare avec la vôtre — une préversion en accepte d'autres,
une version définitive ne se voit proposer que du définitif — rien n'est envoyé, rien n'est
téléchargé, rien n'est installé. La mise à jour se fait à la main :

```sh
git pull && ./install.sh
```

Un atelier qui se réécrit tout seul pendant qu'on travaille dedans n'est
pas un atelier.

Côté dépôt, pousser une étiquette `v…` compile, vérifie (`fmt`, `clippy`,
tous les tests) et publie le binaire avec sa somme SHA-256 ; une
étiquette qui porte un tiret est publiée comme préversion, comme le dit
SemVer et comme le programme le lit.

## Journal des versions

Ce qui change d'une version à l'autre est dans [CHANGELOG.md](CHANGELOG.md).

## Licence

MIT. Voir [LICENSE](LICENSE).

---

## In English

namsdec is a workbench for **learning x86-64 NASM assembly** on Linux,
and for **reading binaries you did not write**. It steps through your
program and, at every step, says what the machine is about to do and what
it just did: the effective address an operand resolves to and the byte
sitting there, the old value beside the new one, whether a conditional
jump will be taken and by which rule, a system call written out the way
you would write it in C, the stack read as a stack, where it spends its
time — a heat band in the gutter and a Profile tab ranking the lines —
and what to do when it faults.

It also reads a binary's armour — PIE, NX, RELRO, stack canary, FORTIFY,
IBT, W+X segments, symbols, libraries — stating for each what that state
actually changes, and copies the whole analysis out as Markdown.

Interface in French and English. MIT licensed: what you understand here
is meant to be passed on.
