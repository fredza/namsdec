# Fonctionnalités supplémentaires

Ces fichiers décrivent des **programmes** : un désassembleur, un
décompilateur. namsdec leur passe le binaire et montre ce qu'ils
répondent.

> **Ce que cela suppose est dit franchement.** Contrairement à un thème,
> qui n'est qu'une palette, installer une fonctionnalité revient à
> autoriser le lancement de la commande qu'elle nomme. namsdec ne va donc
> jamais en chercher tout seul, ne les exécute **que** sur demande
> explicite, et écrit la commande complète avant de la lancer.

## En trois gestes

1. **Installer le programme** sur votre machine — voir plus bas, c'est la
   seule étape que namsdec ne fait pas à votre place.
2. **Allumer le module** : Préférences ▸ Modules ▸ « Décompilation par un
   outil extérieur ». Éteint par défaut.
3. **Déclarer l'outil** : Préférences ▸ Outils ▸ **Déclarer**. L'écran dit
   lesquels sont présents sur votre machine, lesquels manquent, et ce
   qu'il faut taper pour les obtenir.

Puis **Outils ▸ Décompiler…**, choisir l'outil, lire la commande, cliquer
« Lancer ». L'avancement s'affiche dans la barre d'état et dans le titre
de la fenêtre.

---

## Laisser namsdec s'en charger

Depuis la 0.2.4-beta.1, namsdec sait aller chercher **Rizin + rz-ghidra**
et **RetDec** lui-même : Préférences ▸ Modules ▸ « Aller chercher les
outils et les construire », puis le bouton **Installer…** à côté de
l'outil, dans Préférences ▸ Outils.

Ce module est éteint par défaut, et plus franchement que les autres :
les autres affichent, celui-ci **télécharge du code depuis Internet et
l'exécute**. Trois règles l'encadrent :

- **Tout est montré avant d'être fait.** La fenêtre liste les commandes,
  dans l'ordre, telles qu'elles seront lancées — puis chacune est
  réaffichée au moment où elle part, avec ce qu'elle répond.
- **Rien ne sort du dossier de namsdec.** Pas de `sudo`, rien dans
  `/usr` : tout va dans `~/.local/share/namsdec/tools/`, et se retire en
  effaçant ce dossier.
- **Rien n'est embarqué.** `curl` télécharge, `tar` extrait, `meson` et
  `cmake` construisent. namsdec appelle les outils du système, comme il
  le fait déjà pour `nasm` et `ld` — le programme n'en grossit pas.

L'empreinte SHA-256 de ce qui est téléchargé est **calculée et
affichée** : comparez-la à celle que publie le projet. Elle n'est pas
comparée à une valeur écrite dans namsdec, parce qu'une constante que
personne n'a vérifiée serait pire qu'une absence.

Ce que namsdec **ne fera pas** : installer un paquet du système. `dnf` et
`apt` demandent des droits d'administrateur, et il ne les prendra pas.
Pour `radare2`, `objdump` et `ndisasm`, la commande est affichée et c'est
à vous de la taper — voir ci-dessous.

## Installer les décompilateurs à la main

### radare2 — le plus simple, aucun greffon

Son pseudo-code (`pdc`) est moins fidèle qu'un vrai décompilateur, mais il
ne demande rien d'autre que lui-même.

```sh
sudo dnf install radare2          # Fedora
sudo apt install radare2          # Debian, Ubuntu
```

### Rizin + rz-ghidra — le décompilateur de Ghidra

C'est le meilleur résultat qu'on obtienne sans installer Ghidra lui-même.
**Rizin seul ne décompile pas** : la commande `pdg` vient du greffon
`rz-ghidra`, et sans lui l'outil est inutilisable pour cela. namsdec le
vérifie et affiche « incomplet » plutôt que de vous laisser essayer.

```sh
sudo dnf install rizin rizin-plugin-ghidra        # Fedora
sudo apt install rizin                            # Debian, Ubuntu — le greffon
                                                  # n'y est pas empaqueté
```

Si votre distribution n'empaquette pas le greffon, il s'installe par le
gestionnaire de paquets de Rizin :

```sh
rz-pm -U                          # met à jour la liste
rz-pm -i rz-ghidra                # installe le greffon
rizin -qc 'pdg?' --  malloc://16  # doit répondre autre chose qu'une erreur
```

### RetDec — du C compilable

Le décompilateur d'Avast. Lent — comptez la minute sur un gros binaire —
et le seul de cette liste à rendre du C qu'un compilateur accepte.

Il n'est empaqueté nulle part : on prend une version publiée.

```sh
# https://github.com/avast/retdec/releases
tar xf retdec-v5.0-linux-64b.tar.xz -C ~/.local/opt/
export PATH="$HOME/.local/opt/retdec/bin:$PATH"   # à mettre dans ~/.bashrc ou ~/.zshrc
retdec-decompiler --version                      # doit répondre
```

RetDec écrit son résultat **dans un fichier** plutôt que sur la sortie
standard : c'est pourquoi sa description contient `{out}`, que namsdec
remplace par un fichier temporaire dont il lit ensuite le contenu.

### objdump et ndisasm — déjà là

`objdump` vient des binutils, `ndisasm` de nasm : les deux sont sur votre
machine si namsdec fonctionne, puisqu'il lui faut déjà `nasm` et `ld`.

```sh
sudo dnf install binutils nasm    # Fedora
sudo apt install binutils nasm    # Debian, Ubuntu
```

---

## Ceux qui sont ici

| Fichier | Programme | Ce qu'il rend |
|---|---|---|
| `objdump.json` | `objdump` | des instructions, syntaxe Intel, sections comprises |
| `ndisasm.json` | `ndisasm` | des instructions, sans se soucier des sections |
| `radare2.json` | `r2` | du pseudo-code, sans greffon |
| `rizin-ghidra.json` | `rizin` | du C, par le décompilateur de Ghidra |
| `retdec.json` | `retdec-decompiler` | du C compilable |

Ces fichiers sont **engendrés** par le code qui connaît ces outils
(`cargo run -p namsdec-disasm --example export_tools`) : un argument ne
peut donc pas diverger entre ce que namsdec lance et ce que ce dossier
propose de copier.

## Le format

```json
{
  "id": "mon-outil",              identifiant : lettres, chiffres, tirets
  "name": "Mon outil",            ce qui s'affiche dans la liste
  "command": "mon-programme",     le programme à lancer
  "args": ["-d", "{file}"],       ses arguments
  "install": "sudo dnf install …" ce qu'il faut taper pour l'obtenir (facultatif)
}
```

Deux marqueurs dans les arguments :

- **`{file}`** — obligatoire. namsdec y met le chemin d'un fichier
  temporaire contenant le programme lié. C'est ce que l'outil doit lire.
- **`{out}`** — facultatif. namsdec y met le chemin d'un fichier de
  sortie, et lit ce fichier une fois l'outil terminé. Nécessaire pour les
  outils qui écrivent leur résultat ailleurs que sur la sortie standard.

Sans `{out}`, c'est la sortie standard qui est lue — et la sortie
d'erreur quand elle est vide, parce que beaucoup d'outils y écrivent leur
refus.

**Les arguments sont passés tels quels au programme** : aucun interpréteur
de commandes n'est en jeu, donc rien dans un nom de fichier ne peut être
relu comme une commande. Vous ne pouvez pas écrire `foo | bar` ni
`$(…)` — et c'est voulu.

## Les limites

- **Deux minutes**, et l'outil est arrêté pour de bon. Un décompilateur
  qui ne rend pas la main ne bloquera pas namsdec.
- **Quatre mégaoctets** de sortie, au-delà de quoi le texte est coupé :
  ce n'est plus un listing qu'on lit, c'est un vidage.
- Le travail se fait dans un fil à part : l'interface reste utilisable
  pendant ce temps.

---

## Deux dossiers, deux questions

namsdec lit **deux** dossiers, et non un seul :

| Dossier | Ce qu'on y range | Où ça se voit |
|---|---|---|
| `~/.local/share/namsdec/disassemblers/` | un outil qui rend un **listing**, une instruction par ligne | il devient le moteur du panneau principal |
| `~/.local/share/namsdec/decompilers/` | un outil qui rend du **pseudo-code** | Outils ▸ Décompiler… |

Le format des fiches est le même ; c'est le dossier qui dit à quoi elles
servent. Une fiche rangée dans le mauvais dossier est **ignorée sans
message** — namsdec ne sait pas qu'elle existe.

## Brancher son propre outil : l'exemple de DZF

[DZF](https://github.com/…) est un désassembleur x86-64 écrit à la main
en C++23, sans aucune dépendance. C'est exactement le genre d'outil pour
lequel ce format existe : namsdec n'a rien à savoir de lui, sinon la
commande à lancer.

Ce qu'il a fallu pour le brancher — et rien de plus :

1. **Accepter un chemin de fichier en argument.** C'est la seule
   exigence réelle. Un outil qui lit son entrée au clavier ne peut pas
   être appelé par un autre programme : il attendrait une frappe qui ne
   viendra pas, et namsdec finirait par l'arrêter au bout de son délai.
   DZF garde son mode interactif quand on l'appelle sans argument, et
   désassemble le fichier nommé sinon.
2. **Écrire le listing sur la sortie standard.** Une ligne par
   instruction. namsdec ne lui impose aucun format : il montre ce qui
   vient.
3. **Poser la fiche** dans `disassemblers/` :

```json
{
  "id": "dzf",
  "name": "DZF — désassemblage x86-64 (maison)",
  "command": "/chemin/vers/x64-disassembler",
  "args": ["--base", "0x401000", "{file}"],
  "install": "cd DZF/x64-disassembler && cmake -S . -B build && cmake --build build"
}
```

La fiche livrée dans ce dossier (`dzf.json`) nomme son programme
`x64-disassembler` tout court : elle marche si vous mettez le binaire
dans votre `PATH`. Sinon, mettez-y le chemin complet, comme ci-dessus.

**Ce que DZF ne sait pas encore**, et qu'il faut savoir avant de s'en
servir : il ne lit pas l'en-tête ELF. namsdec lui passe le binaire
entier, donc les premiers octets — l'en-tête — sont désassemblés comme du
code, et le début du listing ne veut rien dire. C'est la prochaine étape
de son côté, pas du nôtre : dès qu'il saura trouver ses sections
exécutables, la même fiche continuera de marcher sans y toucher.
