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

**Combien de fois chaque ligne a tourné.** `×5` dans la gouttière en face
du corps de boucle, rien en face de la ligne que le saut enjambe.

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

**Les protections d'un binaire.** PIE, NX, RELRO, canari de pile,
FORTIFY, IBT et pile fantôme, segments W+X, symboles, bibliothèques,
RPATH, et les fonctions qu'il emprunte — chaque entrée de la table de
liaison retrouve son nom, avec le nombre d'appels qui y mènent — avec,
pour chacun, ce que son état change concrètement. Sur
votre programme comme sur un binaire du système. Le rapport se copie en
Markdown, ou s'enregistre, pour être collé dans un compte rendu ou un cours.

Et aussi : documentation de 144 instructions, directives et notions sous **F1** ou
au survol — avec sommaire, renvois d'une fiche à l'autre et export du
manuel entier en Markdown (menu Aide), parce que ce qu'on apprend ici
doit pouvoir se lire ailleurs —, autocomplétion qui ne propose que ce qui est documenté,
motif cyclique de De Bruijn pour savoir de combien un débordement
dépasse (identique à celui de pwntools, donc comparable),
une chronologie où l'on clique un pas passé pour y revenir (registres, indicateurs et mémoire avec), registres et indicateurs qui se posent à la main (« et si ZF valait
1 ? »), points d'arrêt — avec condition (`rcx == 0`), surveillance d'un registre
ou d'une adresse, vérification de la convention d'appel (ce qu'une
sous-routine a emprunté sans le rendre), mémoire protégée par les droits
réels des sections, tas simulé avec `brk`, SSE/AVX et pile x87.

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
et rien ne demande les droits d'administrateur. `./uninstall.sh` défait
exactement cela (`--purge` retire aussi vos réglages et vos séries
d'exercices).

Pour lancer sans installer : `cargo run -p namsdec-gui`.

## Prendre en main

1. **Ctrl+Maj+B** assemble le programme (nasm puis ld, pour de vrai).
2. **Espace s i** avance d'une instruction, **Espace n i** passe par-dessus
   un `call`, **Espace r** lance, **Espace b** revient en arrière d'un pas.
3. **F1** sur un mot du code en donne la fiche ; le survol aussi.
4. Un clic dans la gouttière pose un point d'arrêt ; **Ctrl+F10** exécute
   jusqu'à la ligne du curseur.
5. **Ctrl+Maj+P** ouvre la palette : toutes les commandes s'y trouvent.

Les raccourcis sont dans Préférences ▸ Raccourcis, et se changent.

### Les exercices

Onze des seize exercices portent un **but vérifiable** (`rax == 0xf`) :
le bandeau de l'état du processeur dit « atteint » ou « pas encore », au
lieu de laisser vérifier à l'œil. Une série installée peut en porter
aussi — champ `goal` du fichier JSON.

Un parcours de seize exercices est livré avec le programme — registres,
boucle, bits, comparaison, multiplication, division, ordre des octets,
signé ou non, mémoire, tableaux, chaînes, pile, appels, sortie,
débordement, protection — activable dans
Préférences ▸ Modules. D'autres séries s'installent depuis
un simple fichier JSON, comme les thèmes : Outils ▸ Exercices.

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
| `got`, `plt` | fenêtre Analyse ▸ Fonctions importées |
| `nearpc` | le listing lui-même, RIP dans la gouttière |
| `stepi`, `nexti`, `reverse-stepi` | `Espace s i`, `Espace n i`, `Espace b` |

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
commande vide, `{file}` oublié — est refusé à la lecture plutôt que
découvert au premier essai.

## Le code

Un workspace Rust, une crate par responsabilité :

| crate | ce qu'elle fait |
|---|---|
| `namsdec-core` | registres, indicateurs, mémoire et ses droits |
| `namsdec-asm` | appel de nasm et de ld, lecture de l'ELF produit |
| `namsdec-disasm` | désassemblage x86-64 (iced-x86), syntaxe NASM |
| `namsdec-emu` | l'interpréteur : entiers, SSE/AVX, x87, appels système |
| `namsdec-re` | rétro-ingénierie : protections, références croisées, chaînes, appels système, fragments ROP |
| `namsdec-doc` | les fiches d'instructions et de directives |
| `namsdec-exercises` | les séries d'exercices et leur format |
| `namsdec-theme` | les thèmes, y compris ceux installés depuis le disque |
| `namsdec-explorer` | l'explorateur de fichiers |
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
- Pas de libc : un programme lié dynamiquement s'analyse, mais ne
  s'exécute pas ici (il n'y a pas de vrai noyau derrière).
- Les appels système simulés sont `read`, `write`, `brk` et `exit`, plus
  ceux qui renseignent sans rien changer au monde — `getpid`, `getuid`,
  `geteuid`, `sched_yield`, `nanosleep`, `getrandom`. Les autres sont
  nommés et signalés comme non simulés. `getrandom` rend des octets
  **reproductibles** : c'est ce qu'il faut à un atelier où l'on revient
  en arrière, et ce qu'il ne faut surtout pas à une clé.
- AVX-512, FMA, les ports `in`/`out` et `cmpxchg16b` ne sont pas
  implémentés.

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
you would write it in C, the stack read as a stack, how many times each
line ran, and what to do when it faults.

It also reads a binary's armour — PIE, NX, RELRO, stack canary, FORTIFY,
IBT, W+X segments, symbols, libraries — stating for each what that state
actually changes, and copies the whole analysis out as Markdown.

Interface in French and English. MIT licensed: what you understand here
is meant to be passed on.
