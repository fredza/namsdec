# namsdec

**Un atelier pour apprendre l'assembleur NASM x86-64 — et pour lire les
binaires des autres.**

**A workbench for learning x86-64 NASM assembly — and for reading the
binaries other people wrote.**

> Interface en français et en anglais. Linux, ELF64, syntaxe NASM.
> Interface in French and English. Linux, ELF64, NASM syntax.
> MIT.

### **[Français](#français)** · **[English](#english)**

---

# Français

namsdec exécute votre programme instruction par instruction et vous dit,
à chaque pas, ce que la machine est sur le point de faire et ce qu'elle
vient de faire. Il ne se contente pas d'afficher des registres : il
explique. C'est un outil pédagogique, sous licence MIT, fait pour que ce
qu'on y comprend puisse se transmettre.

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

**Et, pour qui apprend, de quoi les retenir : « Prévoir avant
d'avancer ».** Les trois paragraphes ci-dessus sont une réponse donnée
avant la question : on appuie sur « Pas à pas » pour vérifier ce qu'on
vient de lire, et l'on n'a jamais l'occasion de se tromper — or se
tromper est la seule façon de savoir qu'on n'avait pas compris. En mode
débutant, l'adresse et l'instruction restent à l'écran, la réponse
attend le pas, et « Dernier pas » la révèle **à côté de ce qui a
vraiment changé** :

```
Prochaine instruction
  0x400005  mov ebx, 2
  Qu'est-ce que ça va changer ?   [Montrer]

Dernier pas  0x400000  mov eax, 1
  Ce que ça disait : EAX = 1
  RAX 0x0 -> 0x1
```

« Montrer » donne la réponse sans attendre, d'un clic et sans aveu. Le
réglage est dans Préférences ▸ Modules, sous le mode débutant — il ne
vaut qu'avec lui : un exercice imposé à qui se sert de l'outil tous les
jours serait une taxe.

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

**Les deux jeux de raccourcis, côte à côte.** Préférences ▸ Raccourcis
montre les deux, action par action : l'actif en clair, l'autre pour
comparer, « la même » là où ils ne diffèrent pas. C'est ainsi qu'on voit
ce qu'on vient y chercher — les écarts, ce qu'on a déplacé soi-même, et
deux actions qui répondraient à la même frappe.

**Et l'on circule.** Les adresses des panneaux d'analyse sont
cliquables : une entrée de liaison, une zone libre, une figure, un
fragment — un clic amène le listing dessus. Le suivi de RIP s'arrête au
passage, sans quoi le pas suivant ramènerait le listing là où l'exécution
en est.

**Quand le listing ment.** Un désassemblage lit *linéairement* ; le
processeur, lui, va **où les sauts l'envoient**. Un seul octet-piège
derrière un `jmp` qui l'enjambe, et toute la lecture ligne à ligne part de
travers — c'est l'anti-désassemblage classique, et on le rencontre dans ce
que namsdec est fait pour lire. Le panneau **Figures du code** donne les
**deux lectures côte à côte** et l'octet qui les sépare. Ce qu'on apprend
au passage vaut plus que la figure : le découpage en instructions n'est
pas une propriété des octets, c'est une propriété du point où l'on
commence à lire.

**Ce qui ressort, et pourquoi.** La fenêtre d'analyse ouvre sur
**Signaux** : listing trompé, segment inscriptible *et* exécutable,
entropie qui monte,
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
`~/.local/share/namsdec/modules/` — avec `{out}` dans les arguments
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
`~/.local/share/namsdec/modules/` décrit la commande à lancer
(`objdump`, `ndisasm`, le vôtre). Ceux-là sont des **programmes** : ils
ne s'exécutent que sur demande, et le panneau annonce ce qu'il va lancer
avant de le faire.

**D'où vient cette valeur.** Survoler un registre dit quel pas y a écrit
en dernier, son rang et sa ligne : « écrit au pas -12, ligne 7 :
add rax, rcx ». Le chercher à la main veut dire relire son programme à
l'envers, et se tromper une fois sur deux dès qu'il y a une boucle.

**Des barres d'icônes aux bords.** À gauche l'explorateur et les
exercices, en bas à gauche la console, à droite le panneau du
processeur — chaque icône du côté et à la hauteur où son panneau
s'ouvre. Survolée, elle dit ce qu'un clic fera : « Afficher
l'explorateur », « Cacher l'explorateur », avec le raccourci tel qu'il
est réglé. Elles ne commandent rien que le menu Affichage ne commande
déjà ; ce qu'elles ajoutent est de rendre visible ce qui existe — un
panneau fermé ne laissait aucune trace à l'écran, et l'on ne ferme pas
ce qu'on ne sait pas rouvrir. Chaque panneau porte aussi un tiret au
bout de sa rangée d'onglets, qui le range. La console y porte ses trois
commandes — relancer, arrêter, effacer —, à côté de ce qu'elles
changent. Effacer n'est pas réinitialiser : c'est l'écran qu'on vide, et
le processeur ne bouge pas d'une instruction.

**Le compte des problèmes, sans ouvrir la liste.** Au bord droit de la
rangée des vues : un disque rouge et le nombre d'erreurs, un triangle
orange et le nombre d'avertissements. Un clic ouvre la liste sur le bon
onglet ; les deux chevrons mènent au problème d'avant et à celui
d'après, en bouclant. Il ne paraît que s'il y a quelque chose à compter
— un compteur à zéro en permanence est un endroit de l'écran qu'on
apprend à ne plus regarder.

**La rangée des vues tient sur une ligne.** Ce qui ne rentre pas passe
derrière un bouton qui dit combien — `+5` — et la vue qu'on regarde
reste toujours visible. La rangée passait à la ligne : à 800×520, les
dix vues prenaient trois lignes, quatre-vingt-seize pixels, de quoi
peindre six lignes de code — pour montrer dix onglets dont on en regarde
un. Il restait quatre lignes de programme à l'écran. Les rangées posées
au-dessus d'un panneau haut — le panneau latéral, la calculatrice —
passent à la ligne comme avant : vingt-six pixels n'y coûtent presque
rien.

**Plusieurs fichiers ouverts à la fois.** Une barre d'onglets au-dessus
de l'éditeur, le nom raccourci par le milieu quand il est long — pour que
`boucle_i…ee.asm` et `boucle_i…e_2.asm` restent deux fichiers
différents —, un point sur ce qui n'est pas enregistré. Un exercice
ouvert prend aussi son onglet, sous son titre. Chaque onglet
garde **tout** ce qu'on y faisait : le texte tapé, le curseur, les points
d'arrêt, et l'exécution arrêtée au pas où on l'avait laissée. Ctrl+Tab
passe au suivant, Ctrl+W ferme, et les fichiers reviennent au lancement
d'après.

**L'accueil, quand on a fermé le dernier.** Le nom de l'atelier, puis ce
par quoi on recommence — nouveau fichier, ouvrir, palette de commandes,
exercices — et les fichiers récents, qu'un clic rouvre. Chaque raccourci
montré est celui du tableau des raccourcis, lu et non recopié. Il se
**demande**, par un geste : fermer. Il tient dans l'écran — le blanc du
haut se mesure sur la place, et ce qui ne tient pas encore se rejoint en
descendant. Il ne se déduit pas du tampon vide,
sans quoi il paraîtrait au milieu d'une frappe — on sélectionne tout, on
efface pour recommencer, et l'éditeur disparaîtrait sous les doigts.

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

**Ce que namsdec ne fera pas.** `execve`, `socket`, `connect` : ces
appels système sont nommés mais pas exécutés — namsdec n'a ni second
processus ni réseau à offrir. Le programme qui en contient un
s'arrêterait dessus sans prévenir : c'est dit à l'assemblage, avec la
ligne. Ouvrir un fichier, en revanche, **répond** : « introuvable », ce
qui est la vérité, et le programme suit sa branche d'erreur — souvent la
moitié qu'on était venu voir. La console le dit la première fois.

**Le `int3` qu'un programme se pose à lui-même.** Il installe un
gestionnaire de `SIGTRAP`, exécute `int3`, et regarde s'il a été appelé :
sous un débogueur, c'est le débogueur qui reçoit le point d'arrêt.
namsdec l'appelle, avec un vrai cadre de signal — un gestionnaire qui
réécrit l'adresse de reprise pour sauter par-dessus un leurre est donc
obéi. Sans gestionnaire, l'exécution s'arrête en le disant.

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

Et aussi : documentation de 193 instructions, directives et notions sous **F1** ou
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
[`docs/guide-namsdec-1.0.0-beta.666.pdf`](docs/guide-namsdec-1.0.0-beta.666.pdf)
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

**Le relancer est une mise à jour, pas une réinstallation.** Rien n'est
retiré : vos réglages, les séries que vous avez ajoutées et vos points
d'arrêt restent où ils sont, et le script le dit en tête — « mise à jour
de 0.4.0 vers 0.4.1 » plutôt qu'« installation ». Chaque fichier est
écrit à côté puis renommé, si bien qu'à aucun instant la cible n'est
absente ni incomplète : c'est ce qui manquait au lanceur, que `cat >`
laissait vide une fraction de seconde — assez pour que GNOME, qui élague
les favoris dont il ne trouve plus le `.desktop`, retire l'icône du dock
pendant qu'on installait.

**Et si namsdec est ouvert**, il le reste pendant tout le travail — la
compilation dure quelques minutes, et attendre devant un écran vide n'y
sert à rien. Le binaire est remplacé sous la fenêtre, qui ne s'en aperçoit
pas : elle garde celui qu'elle a ouvert. C'est à la toute fin, une fois
tout en place, que le script la referme proprement et la rouvre sur la
version neuve. `--no-restart` l'en empêche. Une fenêtre qui refuse de
partir — une question posée à l'écran, un travail non enregistré — n'est
jamais forcée : le script le dit et s'arrête là.

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
| `--no-restart` | ne pas refermer ni rouvrir namsdec s'il tourne |
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
2. **Espace s i** avance d'une instruction (`stepi`), **Espace n i** passe par-dessus
   un `call`, **Espace p r** lance, **Espace p b** revient en arrière d'un
   pas. Les deux premiers gardent les lettres de gdb parce que ce sont
   les plus frappés de tout le programme ; le reste des séquences est
   rangé par famille : `p` pour ce qui fait tourner le **programme**,
   `/` pour chercher, `m` pour **marquer**, `v` pour les **vues**, `u`
   pour courir **jusqu'à** quelque chose.
3. **Au niveau débutant, l'éditeur n'a pas de modes** : on clique, on
   tape, ça s'écrit, et la barre d'état ne porte aucune pastille. Au
   niveau avancé, l'éditeur a trois modes, comme vim : **NORMAL** (les mouvements
   `hjkl w b e gg G`, et le compteur `5j`), **INSERT** (`i`, et Échap
   pour sortir) et **VISUEL** (`v` : les mouvements étendent une
   sélection, `y` copie, `d` coupe, Échap renonce).
4. **F1** sur un mot du code en donne la fiche ; le survol aussi.
5. Un clic dans la gouttière pose un point d'arrêt ; **Ctrl+F10** exécute
   jusqu'à la ligne du curseur.
6. **Ctrl+Maj+P** ouvre la palette : toutes les commandes s'y trouvent.

Les raccourcis sont dans Préférences ▸ Raccourcis, et se changent.

**La console n'est pas là au départ, et c'est voulu.** Avant d'avoir
assemblé, elle n'aurait qu'une chose à écrire : qu'elle est vide. Elle
paraît d'elle-même dès qu'elle a quelque chose à montrer — une erreur de
nasm, ou le premier octet que le programme écrit. Refermée en pleine
course, elle le reste ; Affichage ▸ Console l'ouvre à la demande, et le
réglage est gardé.

### Les exercices

**Chacun des cent trente-huit porte sa fiche de théorie.** Ni la consigne,
ni l'indice : la consigne dit ce qu'on demande, l'indice dit où chercher
quand on bloque, et la théorie dit la mécanique sur laquelle l'exercice
repose — ce qu'est une boucle quand le processeur n'en a pas, pourquoi
écrire `eax` efface le haut de `rax`, ce qu'une `convention` d'appel
n'est pas obligée de garantir. On la lit avant d'essayer, et elle
resservira à dix autres étapes.

Elle cite entre accents graves les mots qui ont une fiche, et
l'interface en fait des **renvois cliquables** — vers les instructions
comme vers les notions, ce qui n'était pas le cas de l'énoncé. Rien
n'est tenu à la main : un contrôle vérifie que chaque théorie, dans les
deux langues, cite au moins une fiche qui existe vraiment, qu'elle ne
recopie pas sa consigne, qu'aucune n'est écrite deux fois, qu'elle porte
un **exemple chiffré**, et qu'elle tient dans les bornes de lecture de
sa fenêtre.

Elle est écrite **pour qui débute** : une idée par paragraphe, des
phrases courtes, un exemple avec de vrais nombres, et le piège à la fin.
L'exemple chiffré n'y était pas : sur les cent trente-huit théories,
quatre-vingt-dix-sept ne citaient aucune instruction avec ses opérandes
et cinquante et une ne portaient pas un chiffre. Les deux cent
soixante-seize textes portent maintenant chacun le leur — les valeurs de
départ, ce que chaque instruction en fait, le nombre qui sort —, pris
sur le programme de l'étape et vérifiés contre l'assembleur du dépôt.

Le bouton **La théorie** est dans la rangée des gestes, à côté de
*Un indice* et *La solution* — l'ordre dans lequel on y vient —, et il
ouvre **une fenêtre à elle**. Elle a d'abord été un repli dans la bande,
et les deux états étaient mauvais : dépliée elle poussait le **but** sous
le pli (quatre contrôles l'ont dit), repliée pour cette raison elle ne
s'ouvrait jamais. Une fenêtre règle les deux, et donne enfin la place
qu'un texte pédagogique demande.

Dix-huit exercices demandent de **lire** plutôt que d'écrire : le programme
est donné complet, une question est posée — « que vaut RAX à l'arrêt ? »
—, et l'on répond avant d'exécuter. Le verdict ne donne pas la bonne
réponse : elle s'obtient en exécutant, c'est-à-dire en faisant ce que
l'exercice enseigne. On écrit « 32 », « 0x20 », « 20h » ou
« 0b100000 » — c'est la même réponse, et celui qui la donne en binaire a
compris la même chose. **Et la réponse écrite dans la série est celle
que la machine donne** : un contrôle assemble chacun de ces programmes,
l'exécute et vérifie — une réponse écrite à la main dans un fichier JSON
est une affirmation, passée à l'épreuve de l'émulateur c'est un fait.

Cent douze des cent trente-huit exercices portent un **but vérifiable** (`rax == 0xf`) :
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
aussi — champ `goal` du fichier JSON, et `question` + `answer` pour un
exercice de lecture.

Un parcours de cent trente-huit exercices est livré avec le
programme, rangé en quatre paliers de difficulté croissante :

- **Découverte** (43) — **le programme d'abord** : les quatre lignes de
  la charpente (`global`, `section`, l'étiquette, `hlt`), dont trois ne
  sont pas des ordres ; RIP et le pas à pas, parce qu'un saut n'est rien
  d'autre qu'une adresse écrite là ; `mov` qui copie au lieu de
  déplacer, et dont la destination est à gauche. Puis **les bases** : un
  bit, les poids 1 à 128, l'octet qu'ils forment, le quartet que
  recouvre un chiffre hexadécimal, pourquoi c'est seize et non dix, et
  les trois écritures du même nombre que nasm traduit en les mêmes
  octets. Puis **la structure des registres en entier** : 64, 32, 16 et
  8 bits — soit huit, quatre, deux ou un octet —, les noms hérités des
  huit anciens (`rax`, `eax`, `ax`, `al`) et les suffixes des huit
  numérotés (`r8`, `r8d`, `r8w`, `r8b`), `ah` qui n'est pas le haut, et
  le fait qu'il n'existe pas de tranche de quatre bits. Puis échange,
  mise à zéro, `add` et `sub`. Puis **le saut avant la boucle** : une
  étiquette et un `jmp`, le code qu'un saut en avant laisse derrière
  lui, et ZF tout seul avant que `jnz` n'entre dans une boucle. Puis
  boucles, `loop`, bits, comparaisons, indicateurs, signe. Puis **le
  complément à deux, pas à pas** : fabriqué à la main avec `not` et
  `inc`, les bornes d'un octet et leur asymétrie, le pas qui fait
  basculer 127, et `movzx` contre `movsx` pour élargir sans trahir.
  Puis `inc` contre `add`, `test`, `xchg`/`bswap`, masques de bits,
  constantes calculées à l'assemblage, rotations, `setcc`, `not` et
  `neg`, division signée, coût d'un immédiat. **Dix** de ces étapes sont
  des exercices de **prévision**, placés chacun juste après la théorie
  qu'ils éprouvent — un sens de lecture, une base de numération ou une
  largeur de registre ne s'apprennent pas en les recopiant, mais en
  répondant.
- **Pratique** (33) — décalages, compte de bits, multiplication,
  division, mémoire, tailles d'opérande, ordre des octets, données
  déclarées, tableaux, matrices, recherche, calcul d'adresse, minimum et
  maximum, parité, occurrences, retournement sur place, bornage sans
  branchement, puissances de deux, somme et moyenne, tableau d'octets,
  zones qui se chevauchent (`memmove`), remplissage par `rep stosb`,
  recherche de bit, addition sur 128 bits, table de conversion,
  alignement et trous d'une structure. Puis la **rétro-ingénierie**
  commence, avant même de lire du code : la signature d'un fichier,
  ce qu'un `strings` laisse voir, position dans le fichier contre
  adresse en mémoire. Et trois exercices de **prévision** : multiplier
  par un décalage, ce qu'une boucle a accumulé, lequel des quatre octets
  `[rsi + rcx]` désigne.
- **Structures** (36) — chaînes, instructions de chaîne, comparaison de
  chaînes, texte vers nombre, macros, enregistrements et champs, table
  de saut, choix sans branchement, pile, appels, convention d'appel,
  récursion, Fibonacci, cadre de pile, tableau local, tri, `strlen` par
  `scasb`, mot à l'envers, compte de mots, pile qui retourne, arguments
  au-delà de six, pointeur de fonction, alignement de pile, concaténation, extraction
  d'un mot, récursion terminale, tri par insertion, recherche binaire,
  liste chaînée, transposition. Côté **rétro-ingénierie** : la
  comparaison et le saut qui en dépend, dérouler un calcul plutôt
  que l'exécuter, un aiguillage vu du désassemblage. Deux de
  **prévision** : ce que la pile rend d'abord, et `ja` qui n'est pas
  `jg`.
- **Maîtrise** (26) — sortie, entrée, tas, flottants, SIMD, affichage
  décimal et hexadécimal, majuscules, débordement, protections,
  chiffrement XOR, somme de contrôle, canari posé à la main, choix
  vectoriel par masque, somme vectorielle, troncature contre arrondi,
  table ASCII, affichage d'un négatif, lecture jusqu'à la fin. Et la
  **rétro-ingénierie** de ce qui ne veut pas être lu : chaînes
  cachées par XOR, garde `ptrace`, `int3` et son gestionnaire,
  entropie d'une zone, gadget ROP, et l'octet qui change tout.

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
| `stepi`, `nexti`, `reverse-stepi` | `Espace s i`, `Espace n i`, `Espace p b` |

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

**En mode débutant, la rangée n'en propose que trois** : le pseudo-code,
NASM, et `objdump` en syntaxe Intel — celui qu'on a sur toute machine,
et dont la syntaxe est celle qu'on vient d'apprendre en écrivant du
nasm. Six façons de lire le même code devant quelqu'un qui n'en connaît
aucune ne sont pas un choix, c'est une question de plus. Un moteur
choisi au niveau avancé garde sa place si l'on revient au niveau
débutant : on ne retire pas ce qu'on est en train de lire.

### Ajouter un désassembleur

Le panneau de désassemblage propose deux moteurs livrés — le pseudo-code
et la syntaxe NASM — et tous ceux qu'on lui décrit. Un fichier JSON par
outil, dans `~/.local/share/namsdec/modules/` — tous les outils y vivent,
et c'est le champ `"kind"` qui dit si la fiche rend des instructions
(`"disassembler"`, la valeur par défaut) ou du pseudo-code
(`"decompiler"`) :

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
| `namsdec-re` | rétro-ingénierie : signaux, anti-désassemblage, protections, références croisées, chaînes et indicateurs, familles de comportement, appels système, empaqueteurs, fragments ROP, place libre |
| `namsdec-doc` | les fiches d'instructions et de directives |
| `namsdec-exercises` | les séries d'exercices et leur format |
| `namsdec-theme` | les thèmes, y compris ceux installés depuis le disque |
| `namsdec-explorer` | l'explorateur de fichiers |
| `namsdec-update` | la recherche de version publiée, et la comparaison SemVer |
| `namsdec-gui` | l'interface (egui / eframe) |
| `namsdec-cli` | la commande d'analyse : rapport, JSON, comparaison de deux versions |

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

### Vérifier l'émulateur contre un vrai processeur

Treize mille lignes de sémantique écrites à la main ne se vérifient pas en
les relisant. Un indicateur calculé de travers ne plante pas : il rend une
mauvaise réponse, en silence, et le programme émulé prend une branche qu'il
n'aurait pas prise.

```sh
cargo run --release -p namsdec-emu --example oracle
cargo run --release -p namsdec-emu --example oracle -- "adc rax, rbx"
cargo run --release -p namsdec-emu --example oracle -- --sde      # AVX-512
```

Pour chaque cas, l'oracle sème les seize registres généraux, quatre registres
vectoriels et les indicateurs ; bâtit un petit programme qui pose cet état,
exécute l'instruction et relève son propre état ; le lance sur le processeur
de la machine ; fait partir `namsdec-emu` du même état sur les mêmes octets ;
et compare, registre par registre, indicateur par indicateur.

**Aucune réponse n'est écrite à la main.** On ne dit jamais ce que `adc`
devrait donner — on le demande au processeur. Un cas qu'on n'avait pas prévu
est donc traité comme les autres, et ce sont précisément ceux-là qui trouvent
les défauts.

La moitié du travail est la table des **indéfinis**. Le manuel d'Intel laisse
certains indicateurs à la discrétion du processeur — `OF` après un décalage
d'un nombre autre que 1, presque tout après `bsf`. Les comparer reviendrait à
reprocher à namsdec de n'être pas ce modèle-ci, donc l'oracle connaît la
règle, instruction par instruction. **Toute instruction ajoutée au répertoire
demande qu'on vérifie d'abord sa ligne dans le manuel** : sans cela l'oracle
crie au loup, et personne ne s'en sert plus.

`--sde` passe par [Intel SDE](https://www.intel.com/content/www/us/en/developer/articles/tool/software-development-emulator.html),
qui émule AVX-512 sur n'importe quel x86-64 — utile quand la machine n'en a
pas, ce qui est le cas de la plupart des processeurs grand public depuis que
Intel l'a retiré de ses cœurs hybrides. L'oracle le cherche dans `SDE_PATH`,
dans le `PATH`, puis dans `~/PrgInd`. QEMU ne convient pas : il annonce les
drapeaux `avx512` mais son TCG ne les exécute pas.

L'oracle **ne tourne pas sur l'intégration continue** — il demande `nasm`,
`ld`, un x86-64, et SDE pour AVX-512. Ce qu'il apprend doit donc être retenu
par un test ordinaire dans `namsdec-emu/src/tests.rs`.

### Mesurer ce que le détour sait faire

Le détour refuse ce qu'il ne sait pas relocaliser, et l'interface le dit
site par site. Elle ne dit pas **combien** de sites il refuse — et un outil
qui refuserait presque partout ne servirait à rien sans que personne ne s'en
aperçoive.

```sh
cargo run --release -p namsdec-re --example taux_detour -- tests/binaires/*
```

Le banc planifie un détour à **chaque adresse d'instruction** de chaque plage
exécutable, et compte les réussites contre les refus, par motif et par
mnémonique. Rien n'est écrit : `plan_detour` rend un plan sans toucher au
fichier, ce qui est précisément ce qui permet de le mesurer en masse.

Sur les onze binaires d'épreuve du projet — 3,4 millions de sites — **deux
sur trois acceptent un détour**. L'accès relatif à `rip`, qu'on craignait
bloquant parce qu'il est la forme normale d'un accès à une globale, ne fait
**jamais** refuser. Le seul refus de principe est le saut court, un site sur
dix-huit ; le reste vient de la place libre, pas des instructions.

Il balaie **linéairement**, donc une table de données enchâssée dans du code
est comptée comme du code. C'est assumé : ne mesurer que les fonctions
nommées ne mesurerait rien sur un binaire dépouillé, qui est le cas qui
intéresse. Et il retombe sur les **segments** quand les en-têtes de section
manquent — sans cela, deux des onze binaires rendaient « zéro site ».

## Ce qu'il ne fait pas

- Pas de mode 32 bits, pas d'`int 0x80`.
- **Pas d'éditeur de liens dynamique.** namsdec charge un binaire comme
  le ferait le noyau — segments projetés en pages, relocations
  appliquées, mémoire locale au fil posée, pile de processus écrite avec
  `argc`, `argv`, l'environnement et le vecteur auxiliaire — mais il ne
  charge aucune bibliothèque. Un binaire lié **statiquement** s'exécute
  donc entièrement, bibliothèque C comprise.
- À la place, namsdec **fournit lui-même** cent soixante-dix fonctions,
  écrites en Rust : `printf` et ses cousines, `puts`, `fgets`, `scanf` et
  `sscanf`, les fonctions de chaînes et de mémoire — `strtok` compris,
  qui découpe en place —, les **classes de caractères** et les tables de
  `<ctype.h>` que la glibc lit au lieu d'appeler, `malloc` et sa famille
  — qui prend sa mémoire dans le tas, où on la voit —, et les enveloppes
  des appels système. Lesquelles ajouter n'a pas été deviné : les
  imports de tout le corpus d'épreuve, moins ce qui était déjà fourni,
  classés par le nombre de binaires qui les réclament. Un binaire **dynamique** ordinaire s'exécute donc lui
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
  `fstat`, `lseek`, `ioctl`, `writev`, `prlimit64`, `futex`. S'y
  ajoutent ceux qui touchent aux **fichiers** — `open`, `openat`,
  `stat`, `access` : ils répondent « introuvable », ce qui est la vérité
  puisqu'il n'y a pas de système de fichiers, et le programme suit sa
  branche d'erreur au lieu de s'arrêter devant la question ; la console
  le dit la première fois. Les autres — lancer un processus, ouvrir une
  connexion — sont nommés et signalés comme non simulés : il n'y a
  derrière namsdec ni réseau, ni second processus. `getrandom` rend des octets
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

### L'analyse sans la fenêtre

`namsdec-cli` écrit ce que les panneaux d'analyse montrent, sans ouvrir
d'interface. C'est le même fond, par le même chemin : l'analyse est
sortie de l'interface, et rien n'y est peint.

```sh
namsdec-cli programme                   # le rapport, à l'écran
namsdec-cli programme -o rapport.md     # le rapport, dans un fichier
namsdec-cli programme --rubriques       # ce dont ce fichier a de quoi parler
namsdec-cli programme -s protections    # une seule rubrique
```

**Et ce qu'elle rend possible, qui est sa raison d'être : comparer deux
versions d'un binaire.** Une protection qui disparaît, une chaîne qui
apparaît, une fonction qui grossit — un `diff` d'octets dirait « le
fichier a changé à l'offset 0x15604 », ce qui n'apprend rien.

```sh
namsdec-cli programme.v2 --contre programme.v1
```

```diff
# Ce qui a changé — programme.v1 → programme.v2

2 rubriques ont changé, 17 disent la même chose.

## Protections du binaire — changée

- - √ **NX** pile non exécutable — Du code déposé sur la pile ne s'exécutera pas.
+ - × **NX** pile exécutable — La pile est exécutable : des octets qu'on y écrit
+     peuvent être exécutés tels quels.
+ - × **Segment W+X** 0x0 — Une zone est à la fois inscriptible et exécutable.
- | GNU_STACK | rw- | 0x0 | 0 o |
+ | GNU_STACK | rwx | 0x0 | 0 o |
```

Ci-dessus, **un seul bit a changé dans le fichier** — celui qui rend la
pile exécutable. Un `diff` d'octets aurait dit « un octet a changé » ;
l'analyse en tire trois phrases, et le signal qui va avec.

**Le compte de ce qui n'a pas changé est la moitié utile du rapport** :
« quinze rubriques disent la même chose » est ce qui permet de croire
aux quatre qui ont bougé. Une rubrique qui apparaît ou disparaît entière
est dite comme telle, sans recopier ses trois cents lignes.

En CI, `-s protections` pose la seule question qui compte — les
empreintes, elles, changent à chaque compilation — et **le code de
retour porte la réponse** : `4` quand les deux analyses diffèrent, comme
`diff` rend `1`. Rien à relire pour le savoir.

```sh
namsdec-cli programme.v2 -c programme.v1 -s protections || echo "une protection a bougé"
```

Le rapport est **reproductible** : deux appels sur le même fichier
rendent le même texte, parce que la commande repart des octets. C'est
aussi ce qu'elle ne fait pas — elle ne porte rien d'une session (pas de
nom donné à une fonction, pas de détour calculé), et elle ne lance aucun
outil extérieur : les rubriques `objdump`, `radare2`, Ghidra et RetDec
restent muettes, pour qu'un rapport ne parle jamais de la machine qui l'a
produit.

Ses codes de retour sont faits pour une chaîne d'intégration : `0` le
rapport est écrit (et, en comparaison, rien n'a changé), `1` la ligne de
commande est fautive, `2` le fichier ne se lit pas, `3` il se lit mais il
n'y a rien à analyser — un artefact vide ou tronqué, qui sans cela
passerait pour analysé —, `4` les deux analyses comparées diffèrent.
`namsdec-cli --aide` dit tout le reste.

### Les mises à jour

« À propos ▸ Rechercher une mise à jour » demande à GitHub les versions
publiées et compare avec la vôtre — une préversion en accepte d'autres,
une version définitive ne se voit proposer que du définitif — rien n'est envoyé, rien n'est
téléchargé, rien n'est installé. La mise à jour se fait à la main :

```sh
git pull && ./install.sh
```

Vous pouvez le lancer avec namsdec ouvert : la fenêtre reste là pendant la
compilation, et se rouvre d'elle-même sur la version neuve à la fin.

Un atelier qui se réécrit tout seul pendant qu'on travaille dedans n'est
pas un atelier.

Côté dépôt, pousser une étiquette `v…` compile, vérifie (`fmt`, `clippy`,
tous les tests) et publie le binaire avec sa somme SHA-256 ; une
étiquette qui porte un tiret est publiée comme préversion, comme le dit
SemVer et comme le programme le lit.

## Journal des versions

Ce qui change d'une version à l'autre est dans [CHANGELOG.md](CHANGELOG.md).

## Licence

Écrit par **Frédéric Zawalski**.

MIT. Voir [LICENSE](LICENSE).

---

# English

namsdec runs your program one instruction at a time and tells you, at
every step, what the machine is about to do and what it just did. It does
not merely display registers: it explains. It is a teaching tool, MIT
licensed, built so that what you understand here can be passed on.

---

## What it shows that the others do not

**The next instruction, resolved.** `mov al, [rsi + rcx*4 + 8]` does not
tell you *where* that points: you have to hold three registers in your
head and do the arithmetic. namsdec does it before execution — the
effective address, its label in your own source (`tampon+2`), and the
byte sitting there.

**The last step, with the value from before.** `RCX 0x5 → 0x4` under the
`dec rcx` that did it, and the flags it changed beside it. After a `cmp`,
the flags *are* the result: the instruction touches no register at all.

**The conditional jump, before it goes.** "The jump will be taken", the
rule it follows (`CF = 0 and ZF = 0`), and whether the comparison reads
its numbers as signed. `ja` and `jg` both say "greater" and answer the
opposite on the same bytes: that is exactly where people get it wrong.

**And, for the learner, a way to make it stick: "predict before you
step".** The three paragraphs above are an answer handed over before the
question was asked — you press *Step* to confirm what you just read, and
never get the chance to be wrong. Being wrong is the only way to find out
you had not understood. In beginner mode the address and the instruction
stay on screen, the answer waits for the step, and *Last step* reveals it
**next to what actually changed**:

```
Next instruction
  0x400005  mov ebx, 2
  What is this going to change?   [Show]

Last step  0x400000  mov eax, 1
  What it said: EAX = 1
  RAX 0x0 -> 0x1
```

*Show* gives the answer straight away, one click, no confession. The
setting lives in Preferences ▸ Modules, under beginner mode — it only
makes sense there: an exercise forced on somebody who uses the tool daily
would be a tax.

**The system call written out.** `syscall` carries no operand at all —
everything is in the registers, by a convention you either know or go
looking for somewhere else:

```
write(fd = 0x1, buf = 0x402000 → "salut\n", count = 0x6)
```

**The stack read as a stack.** One eight-byte word per line, RSP and RBP
marked, and any value pointing into the code named for what it is: a
return address, with its label. What sits above RSP is greyed out —
those bytes are still there, but the next `push` overwrites them.

**When the listing lies.** A disassembler reads *linearly*; the processor
goes **where the jumps send it**. One trap byte behind a `jmp` that steps
over it, and the whole line-by-line reading goes askew — classic
anti-disassembly, and you meet it in the very things namsdec is built to
read. The **Code shapes** panel gives you **both readings side by side**
and the byte that separates them. What you learn on the way is worth more
than the finding itself: where instructions begin is not a property of
the bytes, it is a property of where you start reading.

**What stands out, and why.** The analysis window opens on **Signals**: a
fooled listing, a segment both writable *and* executable, rising entropy,
a packer recognised by name, anti-debugging, missing protections, flags
hiding in strings. Nothing new is computed there — it is what the other
panels already found, gathered to say where to start. Each line gives a
**fact** (an address, a name, a measurement), **what that fact changes**,
and **the panel** that explains it. Nothing is added up into a score: a
score would look like a verdict, and it is the reader who concludes.

**What the binary can do, in six families.** Imports and system calls are
listed everywhere; here they are also *sorted* — network, execution,
anti-debugging, memory, cryptography, files. The question is not "which
functions" but "what does this binary do", and three lit families are
already a portrait. A `socket` is a `socket` whether it arrives through
the relocation table or as number 41 in RAX — and a program trying to go
unnoticed uses precisely the second.

**A decompiler, if you have one.** The **Decompilation** module
(Preferences ▸ Modules, off by default) runs the tool of your choice. Six
are known in advance — `objdump`, `ndisasm`, radare2's pseudo-code,
**Rizin + rz-ghidra**, **RetDec** and **DZF** — with the arguments that
work. Each card answers **two questions**, and you need both for the tool
to be of use: is the program **installed on this machine**, and is it
**enabled in namsdec**? The two are independent. Enabling one **writes a
file of a few lines**: namsdec downloads nothing and launches nothing on
its own.

namsdec can also go and fetch them: Preferences ▸ Modules ▸ *Fetch the
tools and build them*. That module is off by default and more plainly so
than the others — the others display, this one downloads code and runs
it. So the window shows **the whole recipe before anything starts**, and
the log is written live. No step calls `sudo`, no step goes through a
shell, and nothing is placed outside `~/.local/share/namsdec/tools/`.

**Adding your own** is a JSON file dropped in
`~/.local/share/namsdec/modules/` — with `{out}` in the arguments if the
tool writes its result to a file rather than to standard output. Three
locks, because this means running a program on your machine: the module
is enabled by hand, the window opens on request, and **the command is
spelled out in full before it is run**. The work happens alongside — the
interface does not freeze — and a tool that has not come back after two
minutes is stopped rather than waited on forever. Progress is readable in
the status bar **and in the window title**, so in your system's task bar.

**Where this value came from.** Hovering a register says which step wrote
it last, its rank and its line: "written at step -12, line 7: add rax,
rcx". Finding that by hand means reading your program backwards, and
getting it wrong every other time as soon as there is a loop. When there
is no history — a binary calling an absent library does not run — namsdec
walks the **code** backwards to the source instead, stopping at the first
jump: beyond it several paths lead here, and "the last write" no longer
has a single meaning.

**The shape of the program, not its sequence of instructions.** `Space g`
opens the **flow graph**: each box is a block you can only enter from the
top and leave from the bottom, arrows show the taken jump (green), the
untaken one (red) and the call (blue), and a loop shows up by going back
up. The block the program is in is brought forward, a click takes you
there in the editor, and Ctrl+wheel zooms out to see the overall shape.
That is what you reconstruct in your head while reading a listing — and
what a beginner cannot see yet.

**Where the program spends its time.** A heat band in the gutter — green
for a line walked once, red for the hottest, yellow and orange in between
— with the exact count on hover and a legend under the editor. The
console's **Profile** tab turns it into a ranking: the number of passes,
the share of the total, the line itself, and a click that takes you
there.

**How do you even get there?** In the flow graph, a field takes an
address — the one where the program accepts, say — and gives you **the
route to reach it**, with the conditions to satisfy in order: this `jne`
must not be taken, that one must. The blocks along the path are brought
forward in the drawing. It is the shortest route, and namsdec says when
others exist rather than letting you believe in a proof.

**What the code does that is worth noticing.** The **Shapes** module
picks out constants that are really text (`mov ecx, 0x48555648` →
`"HVUH"`), length checks, comparisons disguised as XOR — `a XOR b` is
zero only if the two are equal, so the constant *is* the expected value —
string comparisons and jump tables. Families are sorted **from the rarest
to the most common**: in a binary of sixty thousand instructions, seventeen
hundred zero tests are punctuation, and four XOR comparisons are a lead.

**Where to start reading.** The **Functions** module recovers `main` in a
stripped binary — the startup code hands its address to
`__libc_start_main`, in RDI — then cuts the code at call targets, with
the number of callers for each. And strings from languages without a
terminating zero (Rust, Go, Zig) are **cut by the code**, where a byte
scan reads only one blob: there the address travels with its length.

**A binary's armour.** PIE, NX, RELRO, stack canary, FORTIFY, IBT and
shadow stack, W+X segments, symbols, libraries, RPATH, and the functions
it borrows — each relocation entry gets its name back, with the number of
calls leading to it — with, for each, what its state actually changes. On
your own program as on a system binary. The report copies out as
Markdown, or saves, to be pasted into a write-up or a course.

**And also:** documentation for 193 instructions, directives and concepts
under **F1** or on hover — with a table of contents, cross-references and
an export of the whole manual as Markdown (Help menu), because what you
learn here must be readable elsewhere — completion that only offers what
is documented, a De Bruijn cyclic pattern to measure how far an overflow
went (identical to pwntools', hence comparable), a command palette that
also reaches your program's labels, a timeline where you click a past
step to return to it (registers, flags and memory with it), registers and
flags you can set by hand ("what if ZF were 1?"), breakpoints — with a
condition (`rcx == 0`), watching a register or an address, calling
convention checks (what a subroutine borrowed without giving back),
memory protected by the real permissions of the sections, a simulated
heap with `brk`, SSE/AVX and the x87 stack.

## Plugins

The [`plugins/`](plugins/) directory holds what can be added to namsdec
without recompiling it, in two families:

- [**Themes**](plugins/themes/) — colours, and nothing else. A theme does
  not execute.
- [**Extra features**](plugins/fonctionnalites/) — the description of a
  **program** to run: `objdump`, `ndisasm`, `radare2`, Rizin + rz-ghidra,
  RetDec. Each with its installation instructions and the arguments that
  work.

The distinction is not a ranking, it is a matter of trust: namsdec never
goes looking for a program on its own, runs one only on an explicit
request, and writes the full command out before running it.

## The guide

A **PDF user guide** ships with every release:
[`docs/guide-namsdec-1.0.0-beta.666.pdf`](docs/guide-namsdec-1.0.0-beta.666.pdf)
— installing, getting started, every panel, stepping, memory, the manual,
the flow graph, the exercise path, reverse engineering, the shortcuts and
the known limits.

It is **produced from the repository** (`python3 docs/guide.py`): the
version, the number of exercises, the number of documentation cards and
the shortcut table come from the sources, never from a copy that would
age.

## Installing

You need **Rust** (`cargo`), **nasm** and **ld** (binutils):

```sh
# Fedora
sudo dnf install nasm binutils
# Debian, Ubuntu
sudo apt install nasm binutils
```

Then:

```sh
git clone https://github.com/fredza/namsdec.git
cd namsdec
./install.sh
```

`install.sh` compiles, puts the binary in `~/.local/bin`, and the icon
and launcher where the desktop looks for them. Nothing is written outside
`$HOME` and nothing asks for administrator rights.

**Running it again is an update, not a reinstall.** Nothing is removed:
your settings, the exercise packs you added and your breakpoints stay
where they are, and the script says so up front. Each file is written
beside its target and then renamed, so the target is never missing nor
half-written.

**And if namsdec is open**, it stays open for the whole job — compiling
takes a few minutes, and waiting in front of an empty screen helps
nobody. The binary is replaced under the window, which does not notice.
Only at the very end, once everything is in place, does the script close
it cleanly and reopen it on the new version. `--no-restart` prevents
that. A window that refuses to go — a question on screen, unsaved work —
is never forced: the script says so and stops there.

`./install.sh --help` lists the options. The most useful ones:

| option | what it does |
|---|---|
| `--dry-run`, `-n` | say what would be done, write nothing |
| `--debug` | install the unoptimised build, ready in seconds |
| `--prefix DIR` | where icons, launcher and exercises go |
| `--bin-dir DIR` | where the binary goes |
| `--no-desktop` | no launcher, no icons |
| `--no-dock` | do not pin to the dock (GNOME) |
| `--no-path` | do not touch the shell configuration file |
| `--no-exercises` | do not install the bundled pack |
| `--no-strip` | keep debug symbols (two megabytes more) |
| `--no-build` | do not compile: install the binary already built |
| `--no-restart` | do not close and reopen namsdec if it is running |
| `--quiet`, `-q` | say only what matters |
| `--no-color` | no colours — `NO_COLOR` is honoured too |

**A lighter binary**: `cargo build --release --no-default-features` drops
the update check, and the TLS stack with it — **16.0 → 14.4 MB** once
stripped, ten per cent. That is what a distribution packager wants:
updates are their job, not the application's. The menu then does not show
the entry, rather than offering a button that can do nothing.

`./uninstall.sh` undoes exactly that, and takes `--prefix`, `--bin-dir`,
`--dry-run`, `--quiet` and `--no-color` the same way. `--purge` also
removes your settings and exercise packs — without it they are kept,
because people usually uninstall in order to reinstall.

To run without installing: `cargo run -p namsdec-gui`.

## Getting started

1. **Ctrl+Shift+B** assembles the program (nasm then ld, for real).
2. **Space s i** steps one instruction (`stepi`), **Space n i** steps
   over a `call`, **Space p r** runs, **Space p b** steps one back. The
   first two keep gdb's letters because they are the most hit keys in the
   whole program; the rest of the sequences are sorted by family: `p` for
   what runs the **program**, `/` to search, `m` to **mark**, `v` for the
   **views**, `u` to run **until** something.
3. **At beginner level the editor has no modes**: you click, you type, it
   is written, and the status bar carries no badge. At advanced level the
   editor has three modes, like vim: **NORMAL** (the `hjkl w b e gg G`
   motions, and the `5j` counter), **INSERT** (`i`, Escape to leave) and
   **VISUAL** (`v`: motions extend a selection, `y` copies, `d` cuts).
4. **F1** on a word in the code gives its card; hovering does too.
5. A click in the gutter sets a breakpoint; **Ctrl+F10** runs to the line
   under the cursor.
6. **Ctrl+Shift+P** opens the palette: every command is in there.

The shortcuts live in Preferences ▸ Shortcuts, and can be changed.

**The console is not there at the start, and that is deliberate.** Before
you have assembled anything it would have one thing to say: that it is
empty. It appears by itself as soon as it has something to show — an
error from nasm, or the first byte the program writes.

### The exercises

A path of **one hundred and thirty-eight exercises** ships with the
application, in four levels: **Discovery** (43) — the program first,
**Practice** (33) — shifts, bit counting, multiplication, **Structures**
(36) — strings, string instructions, **Mastery** (26) — output, input,
the heap, floating point, SIMD. Each one carries its own **theory card**:
not the brief, not the hint — the notion the exercise is about, with
worked numbers.

**The path is not in the binary**: it is a data file, and yours is added
the same way.

### If you come from pwndbg

The shortcut set follows gdb where it matters, and the panels answer the
same questions — with the explanation added. The cyclic pattern is
pwntools', so what you measure here is comparable with what you measure
there.

### Three ways to read the same instruction

Pseudo-code (`RAX = RAX + RCX` rather than `add rax, rcx`), NASM syntax,
and the system's own syntax — the one gdb, objdump and as speak. **In
beginner mode the row offers only three**, because a choice you cannot
yet judge is not a choice.

### Adding a disassembler

A JSON file in `~/.local/share/namsdec/modules/` describes the command to
run. **If the tool does not read ELF**, two more markers let it off:
namsdec hands it the executable bytes and their address instead.

## The code

A Rust workspace, one crate per responsibility:

| crate | what it does |
|---|---|
| `namsdec-core` | registers, flags, memory and its permissions |
| `namsdec-asm` | calling nasm and ld, reading the ELF they produce |
| `namsdec-disasm` | x86-64 disassembly (iced-x86), NASM syntax |
| `namsdec-emu` | the interpreter: integers, SSE/AVX, x87, system calls |
| `namsdec-re` | reverse engineering: signals, anti-disassembly, protections, cross-references, strings and flags, behaviour families, system calls, packers, ROP gadgets, slack space |
| `namsdec-doc` | the instruction and directive cards |
| `namsdec-exercises` | the exercise packs and their format |
| `namsdec-theme` | the themes, including those installed from disk |
| `namsdec-explorer` | the file explorer |
| `namsdec-update` | looking up the published release, and SemVer comparison |
| `namsdec-gui` | the interface (egui / eframe) |
| `namsdec-cli` | the analysis command: report, JSON, comparing two versions |

The domain crates know nothing of the interface language: their errors
carry an English message and keep their values in structured variants,
which the interface translates.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
```

The tests do not merely check functions: they assemble and run real
programs with nasm and ld, draw the interface off-screen and read what
was painted. Every program shipped with the application, and every
example in the cards, must assemble, link without a warning, and run to
the end.

### Checking the emulator against a real CPU

Thirteen thousand hand-written lines of semantics cannot be verified by
re-reading them. A flag computed wrongly does not crash: it returns a
wrong answer, silently, and the emulated program takes a branch it should
not have. So the emulator is compared against the real processor, through
gdb.

## What it does not do

- No 32-bit mode, no `int 0x80`.
- **No dynamic linker.** namsdec loads a binary the way the kernel would
  — segments mapped into pages, relocations applied, thread-local memory
  laid down, the process stack written with `argc`, `argv`, the
  environment and the auxiliary vector — but it loads no library. A
  **statically** linked binary therefore runs in full, C library
  included.
- Instead, namsdec **provides** one hundred and seventy functions of its
  own, written in Rust: `printf` and its cousins, `puts`, `fgets`,
  `scanf` and `sscanf`, the string and memory functions — `strtok`
  included, which cuts in place — the **character classes** and
  `<ctype.h>` tables the glibc reads instead of calling, `malloc` and its
  family — which takes its memory from the heap, where you can see it —
  and the system call wrappers. An ordinary **dynamic** binary therefore
  runs too, from prompt to verdict; it stops by **naming** the first
  function it borrows that namsdec does not provide.
- Those functions obey the same rules as everything else: the simulated
  memory permissions apply (a `strcpy` into `.rodata` fails, as it would
  on the real machine), each call counts as **one step**, and what it
  writes is undone by *step back*.
- The simulated system calls are the ones that *do* the work — `read`,
  `write`, `brk`, `mmap`, `mprotect`, `exit` — the ones that report
  without changing the world — `getpid`, `getuid`, `geteuid`,
  `sched_yield`, `nanosleep`, `getrandom`, `uname`, `clock_gettime` — and
  those of a C library's startup: `arch_prctl`, `set_tid_address`,
  `fstat`, `lseek`, `ioctl`, `writev`, `prlimit64`, `futex`. Plus the
  ones that touch **files** — `open`, `openat`, `stat`, `access`: they
  answer "not found", which is the truth since there is no filesystem,
  and the program follows its error branch instead of stopping in front
  of the question. The others — starting a process, opening a connection
  — are named and reported as not simulated: behind namsdec there is
  neither network nor second process. `getrandom` returns
  **reproducible** bytes: exactly what a workshop where you step backwards
  needs, and exactly what a key must never have.
- AVX-512, FMA, the `in`/`out` ports and `cmpxchg16b` are not
  implemented.

### The window

namsdec draws its own title bar: the menus stand in for it, and the three
buttons are **at the head**, on the left — close, minimise, maximise.
Closing therefore occupies the corner of the screen, the angle a mouse
thrown without aiming reaches. Preferences ▸ General hands the desktop
decoration back to whoever prefers it, and the setting applies without
restarting.

### Leaving the workshop

File ▸ **Export the binary** writes the program exactly as `ld` produced
it — an executable ELF, the one the command line would have given: it
runs in a terminal, reads with `objdump`, compares with what namsdec says
about it. The Diagnostics tab shows the **two commands actually run**,
ready to copy:

```sh
nasm -f elf64 -o programme.o programme.asm
ld -Ttext 0x400000 -o programme programme.o
```

A workshop that lets nothing out teaches you to use itself, not the
assembler.

### Analysis without the window

`namsdec-cli` writes out what the analysis panels show, without opening
an interface. Same substance, same path: the analysis was moved out of
the interface, and nothing is painted there.

```sh
namsdec-cli programme                   # the report, on screen
namsdec-cli programme -o report.md      # the report, to a file
namsdec-cli programme --rubriques       # what this file has material for
namsdec-cli programme -s protections    # a single section
```

**And what it makes possible, which is its reason to exist: comparing two
versions of a binary.** A protection that disappears, a string that
appears, a function that grows — a byte `diff` would say "the file
changed at offset 0x15604", which teaches nothing.

```sh
namsdec-cli programme.v2 --contre programme.v1
```

### Updates

*About ▸ Check for updates* asks GitHub for the published releases and
compares with yours — nothing is sent, nothing is downloaded, nothing is
installed. Which releases you are offered is set in Preferences ▸
General: final releases are offered either way, and the box **adds** the
betas on top. Updating is done by hand:

```sh
git pull && ./install.sh
```

A workshop that rewrites itself while you are working inside it is not a
workshop.

## Changelog

What changes from one version to the next is in
[CHANGELOG.md](CHANGELOG.md).

## Licence

Written by **Frédéric Zawalski**.

MIT. See [LICENSE](LICENSE).
