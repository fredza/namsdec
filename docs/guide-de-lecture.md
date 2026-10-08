# Lire le code de namsdec — guide d'entrée

Écrit pour Astra, qui bâtit **DZF** en parallèle et veut apprendre de ce
code. Il vaut aussi pour tout nouveau venu. namsdec est sous licence
**MIT** : lis, copie les idées, adapte.

Deux honnêtetés d'abord, pour apprendre juste :

- Une bonne part de ce code a été écrite par un assistant sous la
  direction de Fred. Tu y verras des **façons de faire**, pas « un style
  d'auteur ». Prends les patterns, pas la calligraphie.
- namsdec est une application **egui** pédagogique en Rust. DZF est un
  moteur **C++23**. La discipline de test et les réflexes de robustesse se
  transfèrent ; les détails d'interface, non.

## La carte

Onze crates. Par taille et par rôle :

| Crate | Rôle |
|---|---|
| `namsdec-core` | le CPU simulé, les registres, les flags, la mémoire |
| `namsdec-asm` | assembler (via nasm), produire l'ELF, lire les erreurs |
| `namsdec-emu` | l'émulateur : exécuter une instruction, pas à pas, réversible |
| `namsdec-disasm` | désassembler (iced-x86), et les **plugins** d'outils externes |
| `namsdec-re` | **l'analyse de binaires** — treize modules branchables |
| `namsdec-gui` | l'interface (le gros morceau) |
| `namsdec-doc`, `-exercises`, `-explorer`, `-theme`, `-update` | manuel, ateliers, arborescence, thème, mises à jour |

## Par où entrer — dans cet ordre

**Ne commence pas par `namsdec-gui`** (67 000 lignes). Entre par
`namsdec-re`, qui est la frontière où DZF se branchera, et qui est fait de
petits modules indépendants.

1. **`crates/namsdec-re/src/lib.rs`** — la frontière d'intégration, à lire
   en entier. Trois types portent tout :
   - `ReContext` : ce qu'un module **reçoit** (les octets, le
     désassemblage, le CPU, les noms donnés aux fonctions). En lecture
     seule.
   - `ReModule` : ce qu'un module **est** (un `id`, un `title`, un rendu
     `ui`, et `on_binary_loaded`/`on_step` pour calculer au bon moment).
   - `ReRequest` : ce qu'un module **demande** à l'application sans savoir
     le faire lui-même (écrire des octets, aller à une adresse, renommer).
     Le module décrit, l'application décide.
2. **`crates/namsdec-re/src/digest_module.rs`** — le module le plus simple,
   ~150 lignes. Lis-le en entier avec ses deux tests : c'est le patron.
3. **`crates/namsdec-re/src/hex_editor_module.rs`** — un module qui *agit*
   (l'éditeur binaire) : grille, inspecteur, recherche, frappe clavier, le
   tout via `ReRequest::PatchBytes`. Vois comment une fonctionnalité riche
   reste faite de **fonctions pures** éprouvées (`frapper`, `inspecter`,
   `trouver`, `motif`).
4. **Les plugins d'outils externes** —
   `crates/namsdec-disasm/src/plugin.rs` et la recette DZF dans
   `install.rs` (`fn dzf`). C'est par là que `x64-disassembler` et
   `dzf-plugin-engine` entrent dans namsdec.

## Les cinq patterns à retenir

1. **Extraire une fonction pure pour l'éprouver.** Dès qu'une logique est
   non triviale (assembler un octet de deux quartets, interpréter des
   octets, chercher un motif), elle sort de l'`ui` en fonction libre,
   testée sans interface. La glu (egui, C++/Qt) reste mince autour.
2. **Mesurer ce qui est produit, pas relire le code.** Les tests d'UI
   relèvent ce qui est *peint* (`output.shapes`) et le mesurent :
   recouvrement, découpage, contraste, dérive. Un défaut visuel se dit
   alors en une ligne — « tel texte et tel autre au même endroit ».
3. **Un test ne lit jamais l'état de la machine.** Il *sème* ce qu'il veut
   éprouver (un faux relevé d'outils, un faux réglage) au lieu de demander
   au disque ou au PATH. Sinon il passe chez toi et tombe en intégration
   continue, sur une machine neuve.
4. **Ce qu'on réécrit, on le remplace d'un bloc.** `write_atomically` :
   écrire à côté, pousser sur le disque, renommer par-dessus. Et toute
   modification du binaire passe par un **journal réversible** (le patch
   garde l'octet d'avant). Un outil qui ne se défait pas n'est pas un
   outil d'analyse.
5. **Greffer sans toucher au reste.** Un registre (`ReModule` +
   `ReModuleRegistry`) : un module neuf s'ajoute en une ligne
   d'enregistrement, et les invariants de scène le couvrent
   automatiquement. Même esprit côté DZF : des unités qu'on branche.

## Pour le pont namsdec ↔ DZF

Ce qui compte le plus à étudier pour le travail commun :

- La **frontière** `ReContext`/`ReModule`/`ReRequest` ci-dessus : c'est le
  moule dans lequel la sortie de DZF viendra se couler (un panneau
  décompilateur qui lit le JSON de `dzf-plugin-engine`).
- Les **noms partagés** : `ReContext::display_name` et `user_name`. Une
  fonction rebaptisée l'est **partout**. La décompilation DZF devra lire
  cette même table, pour qu'un nom donné dans le graphe d'appels se
  retrouve dans le pseudo-C.
- Le **contrat JSON** entre DZF et namsdec reste à figer ensemble : la
  forme exacte (fonctions, signatures, corps, cross-refs). Les deux côtés
  bâtissent vers le même schéma, versionné, à champs optionnels — sinon
  ça dérive.

## Le réflexe qui tient tout

Avant d'inscrire un défaut, **trace l'appel jusqu'au bout**. Un « bug »
récent — « l'analyse reste périmée après un patch » — n'existait pas :
la fonction soupçonnée déléguait à une autre qui faisait le travail. Un
faux positif dans le registre coûte plus qu'il n'informe.
