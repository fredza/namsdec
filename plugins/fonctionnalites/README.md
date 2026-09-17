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

## Installer les décompilateurs

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
