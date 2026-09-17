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
├── themes/          les thèmes
├── decompilers/     les désassembleurs et décompilateurs
└── exercises/       les séries d'exercices (dont le parcours livré)
```

Installer, c'est copier un fichier JSON dans le bon dossier. Trois façons,
au choix :

- **Depuis namsdec** — Préférences ▸ Plugins pour un thème, Préférences ▸
  Outils pour un décompilateur. C'est la voie normale : elle vérifie le
  fichier avant de l'accepter.
- **À la main** — `cp plugins/themes/ardoise.json ~/.local/share/namsdec/themes/`
- **En les déclarant** — pour les outils connus, le bouton « Déclarer »
  de Préférences ▸ Outils écrit le fichier pour vous, avec les arguments
  qui marchent.

Les plugins survivent aux mises à jour : `install.sh` ne touche qu'au
binaire, aux icônes et au parcours d'exercices livré.

## Écrire le sien

Chaque dossier contient un `README.md` avec le format, champ par champ, et
un exemple commenté. Les deux formats tiennent en une vingtaine de lignes.
