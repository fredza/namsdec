# Thèmes

Un thème est une palette : vingt-quatre couleurs, et rien d'autre.
namsdec lit des nombres et peint avec — il n'y a aucun code à exécuter,
donc aucun risque à en installer un.

## Installer

Depuis namsdec : **Préférences ▸ Plugins ▸ Ajouter un thème**, puis
choisir le fichier. Il est vérifié, copié dans
`~/.local/share/namsdec/themes/`, et disponible aussitôt.

À la main :

```sh
cp ardoise.json ~/.local/share/namsdec/themes/
```

Le thème apparaît au lancement suivant. Pour le retirer : le bouton
« Retirer » à côté de sa vignette, ou supprimer le fichier.

## Ceux qui sont ici

| Fichier | Nom | |
|---|---|---|
| `ardoise.json` | Ardoise | sombre, bleuté, contrasté |
| `solarise-clair.json` | Solarisé clair | clair, d'après la palette Solarized |

namsdec en porte quatre autres, compilés dans le programme : VS Code
Dark+, VS Code Light+, Tokyo Night et Noël — ce dernier sombre et
sobre, épicéa et or de bougie. Ceux-là ne se retirent pas.

## Un thème qui bouge

`"festif": true` donne au thème une petite animation : une douzaine de
flocons qui descendent, en gris très pâle, et par moments une boule
creuse — un cerceau, pour qu'on puisse lire au travers — dans le rouge,
l'or et le vert du thème. C'est fait pour se voir du coin de l'œil et
pas davantage.

Deux choses à savoir avant de le mettre :

- **L'utilisateur a le dernier mot.** Préférences ▸ Général porte une
  case « Animer les thèmes qui le proposent » ; décochée, aucun thème
  ne bouge.
- **Tant qu'une animation tourne, la fenêtre se redessine trente fois
  par seconde.** C'est le prix de toute animation. Un thème destiné à
  une machine sur batterie fait mieux de laisser ce champ à `false`.

Le champ est facultatif : un thème écrit avant qu'il existe reste
valide, et ne se met pas à bouger tout seul.

## Le format

```json
{
  "id": "mon-theme",          identifiant : lettres, chiffres, tirets
  "name": "Mon thème",        ce qui s'affiche dans la liste
  "dark": true,               sombre ou clair — décide du contraste des cadres
  "festif": false,            facultatif : quelques flocons et, de loin
                              en loin, une boule. Voir plus bas.

  "bg":        "#12151c",     le fond de l'éditeur
  "panel":     "#181c26",     les panneaux et les barres
  "surface":   "#1f2430",     ce qui se pose par-dessus : fenêtres, cartes
  "text":      "#d7dce5",     le texte ordinaire
  "dim":       "#7a8496",     le texte secondaire, les filets
  "accent":    "#5aa9e6",     ce qui attire l'œil : RIP, sélection d'onglet
  "good":      "#7bc96f",     réussi, saut pris, ligne froide
  "warn":      "#e3c54a",     avertissement
  "bad":       "#e06c75",     erreur, saut non pris, ligne chaude
  "changed":   "#e2883c",     une valeur qui vient de changer

  "mnemonic":  "#5aa9e6",     coloration : l'instruction
  "number":    "#c792ea",     les nombres
  "label":     "#7bc96f",     les étiquettes
  "comment":   "#5c6577",     les commentaires
  "register":  "#89ddff",     les registres          (facultatif)
  "string":    "#a5d6a7",     les chaînes            (facultatif)
  "directive": "#e3c54a",     les directives nasm    (facultatif)

  "folder":      "#e3c54a",   l'explorateur : dossier fermé
  "folder_open": "#e2883c",   dossier ouvert
  "folder_text": "#d7dce5",   les noms de fichiers
  "header":      "#181c26",   l'en-tête des tableaux
  "selection":   "#2a3040"    le fond du texte sélectionné
}
```

Les trois champs facultatifs sont arrivés après le format : un thème
écrit avant eux reste valide, et namsdec y met la couleur la plus proche
de celles qu'il donne. C'est la seule dérivation automatique du crate, et
elle n'existe que pour ne pas casser ce qui marchait.

Les couleurs s'écrivent `#rrggbb` ou `#rrggbbaa`.
