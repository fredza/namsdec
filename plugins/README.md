# Les plugins de namsdec

Tout ce qui s'ajoute à namsdec sans le recompiler se trouve ici, rangé en
deux familles — et la différence entre les deux n'est pas une question de
classement, c'est une question de confiance :

| Famille | Ce que c'est | Ce que ça peut faire |
|---|---|---|
| [**Thèmes**](themes/) | des couleurs, et rien d'autre | rien : namsdec lit des nombres et peint avec |
| [**Fonctionnalités supplémentaires**](fonctionnalites/) | la description d'un **programme** à lancer | ce que ce programme peut faire sur votre machine |

Un thème ne s'exécute pas. Une fonctionnalité, si — et c'est pourquoi
namsdec ne va jamais en chercher tout seul, ne les lance que sur demande
explicite, et écrit la commande complète avant de la lancer.

## Où ils s'installent

```
~/.local/share/namsdec/
├── modules/         TOUS les outils extérieurs, une fiche par outil
├── themes/          les thèmes
├── exercises/       les séries d'exercices (dont le parcours livré)
└── tools/           ce que namsdec a bâti lui-même (rizin, retdec…)
```

**Un seul dossier pour les modules**, et la fiche dit ce qu'elle rend.
Il y en avait deux — `disassemblers/` et `decompilers/` —, et c'était le
DOSSIER qui portait le sens : la même fiche changeait de nature selon
l'endroit où on la posait, et une fiche recopiée à la main dans le
mauvais dossier était simplement ignorée, sans message. Elle porte
maintenant un champ :

```json
"kind": "disassembler"   ou   "kind": "decompiler"
```

Un **désassembleur** rend un listing, une instruction par ligne, et peut
devenir le moteur du panneau principal. Un **décompilateur** rend du
pseudo-code, dans sa propre rubrique. Le champ absent vaut
`disassembler`, qui est le cas courant.

Les deux dossiers d'avant sont **relus au premier lancement** et leur
contenu recopié vers `modules/` — rien n'y est effacé, et l'ancienne
version continue de les lire. Ce qui s'y trouve encore se déplace à la
main : `mv ~/.local/share/namsdec/decompilers/*.json
~/.local/share/namsdec/modules/`, en ajoutant `"kind": "decompiler"`
dans chacun.

Installer, c'est copier un fichier JSON dans le bon dossier. Trois façons,
au choix :

- **Depuis namsdec** — Préférences ▸ Plugins pour un thème, Préférences ▸
  Outils pour un module. C'est la voie normale : elle vérifie le
  fichier avant de l'accepter.
- **À la main** — `cp plugins/themes/ardoise.json ~/.local/share/namsdec/themes/`,
  `cp plugins/fonctionnalites/objdump.json ~/.local/share/namsdec/modules/`
- **En les déclarant** — pour les outils connus, le bouton « Déclarer »
  de Préférences ▸ Outils écrit le fichier pour vous, avec les arguments
  qui marchent.

Les plugins survivent aux mises à jour : `install.sh` ne touche qu'au
binaire, aux icônes et au parcours d'exercices livré.

## Écrire le sien

Chaque dossier contient un `README.md` avec le format, champ par champ, et
un exemple commenté. Les deux formats tiennent en une vingtaine de lignes.
