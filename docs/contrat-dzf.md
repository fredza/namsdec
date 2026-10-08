# Contrat DZF ↔ namsdec — la décompilation en JSON

Pour Astra. Ce document fixe **ce que DZF doit émettre** pour que namsdec
affiche sa décompilation. Le contrat sous sa forme exécutable — et la
source de vérité — est `crates/namsdec-re/src/dzf.rs` (types serde +
tests) ; ce fichier-ci le dit en français.

But : qu'une fonction comprise dans DZF s'affiche dans namsdec, navigable,
et que les deux côtés grandissent **sans se casser**.

## Comment namsdec appelle DZF

namsdec lance le décompilateur comme un **plugin** : une fiche JSON dans
`~/.local/share/namsdec/decompilers/` donne une commande et des
arguments, et namsdec substitue des gabarits avant de lancer :

| Gabarit | Ce que namsdec y met |
|---|---|
| `{file}` | le chemin du **binaire entier** (ELF complet) |
| `{text}` | le chemin des **octets exécutables seuls** (sans en-tête) |
| `{base}` | l'**adresse de chargement** de ces octets |
| `{out}`  | un chemin où DZF **écrit sa sortie** (sinon namsdec lit stdout) |

Fiche-type pour le moteur de décompilation de DZF :

```json
{
  "id": "dzf-decompile",
  "name": "DZF — décompilation",
  "command": "dzf-plugin-engine",
  "args": ["--input", "{file}", "--base", "{base}", "--output", "{out}"]
}
```

DZF lit le fichier, décompile, et **écrit le JSON ci-dessous** dans
`{out}` (ou sur stdout si la fiche n'a pas de `{out}`).

## Le JSON à émettre

```json
{
  "version": 1,
  "functions": [
    {
      "address": 4198432,
      "name": "verifie_cle",
      "signature": "int verifie_cle(char *saisie)",
      "pseudocode": "int verifie_cle(char *saisie) {\n  return strcmp(saisie, \"42\") == 0;\n}",
      "size": 48,
      "calls": [4198400]
    }
  ],
  "diagnostics": ["sub_401500 : trop de chemins, abandonnée"]
}
```

### Les champs

**Au niveau racine :**

| Champ | Type | Oblig. | Rôle |
|---|---|---|---|
| `version` | entier | non (déf. 0) | la version de CE contrat que DZF a suivie |
| `functions` | liste | non (déf. []) | les fonctions décompilées |
| `diagnostics` | liste de chaînes | non (déf. []) | ce que DZF n'a pas su faire, affiché tel quel |

**Pour chaque fonction :**

| Champ | Type | Oblig. | Rôle |
|---|---|---|---|
| `address` | entier u64 | **OUI** | adresse virtuelle d'entrée — la clé qui relie tout |
| `name` | chaîne | non | le nom que DZF propose (`sub_401020`, ou mieux) |
| `signature` | chaîne | non | la signature sur UNE ligne, si reconstituée |
| `pseudocode` | chaîne | non (déf. "") | le corps en C-isme, multi-lignes. Le cœur. |
| `size` | entier u64 | non | la taille de la fonction en octets |
| `calls` | liste u64 | non | les adresses que la fonction appelle |

## Les règles qui font tenir le contrat

1. **Les adresses sont des adresses VIRTUELLES** (u64), les mêmes que
   celles du listing de namsdec. C'est par l'adresse que namsdec recoud la
   décompilation au reste : le listing, les renvois, et surtout les noms.
2. **`address` est la seule exigence dure.** Une fonction sans adresse est
   refusée (rien ne pourrait la relier). Tout le reste a un défaut sûr.
3. **Les champs inconnus sont IGNORÉS, jamais une erreur.** DZF peut
   ajouter `confiance`, `tags`, `variables`… demain : namsdec les laisse
   passer sans broncher. C'est ce qui permet à DZF de grandir d'abord, et
   à namsdec de les exploiter ensuite. Ne jamais compter sur un refus de
   namsdec pour valider un champ.
4. **`version` monte quand la SÉMANTIQUE d'un champ change** (pas quand on
   en ajoute un — ça, c'est gratuit). namsdec lira la version pour adapter
   sa lecture le jour où ce sera nécessaire.
5. **Le JSON vide est valide** : `{}` ou `{"functions":[]}` veut dire « je
   n'ai rien décompilé », et namsdec le dit, au lieu de planter.

## Ce que namsdec fait de chaque champ

- **`pseudocode`** : affiché en monospace, c'est ce qu'on lit. (À venir :
  les adresses qui y apparaissent deviendront cliquables.)
- **`name`** : un point de départ. Si l'utilisateur a **renommé** cette
  adresse dans namsdec (fonction `display_name`), c'est SON nom qui
  s'affiche, ici comme ailleurs — DZF n'a pas à le savoir ni à le relire.
- **`signature`** : en tête de la fonction, quand elle est là.
- **`calls`** : recoupées avec le graphe d'appels de namsdec.
- **`diagnostics`** : montrés à part, pour qu'une fonction manquante ne
  passe pas pour un vide.

## Pour commencer petit

La v1 utile tient en deux champs : `address` et `pseudocode`. Le reste
enrichit. Émets d'abord ça, namsdec l'affichera ; ajoute `name`,
`signature`, `calls` quand DZF les a. Rien ne casse en route.

## En cas de doute

La vérité est dans `crates/namsdec-re/src/dzf.rs`. Si ce document et ce
code divergent un jour, **c'est le code qui a raison** — et il faut
corriger le document (et sans doute se parler).
