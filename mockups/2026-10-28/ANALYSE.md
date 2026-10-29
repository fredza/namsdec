# Avis sur Namsdec — 28 octobre 2026

**Mon avis : un vrai outil pédagogique, avec un socle technique sérieux, dont le périmètre devient plus difficile à maîtriser que l’idée de départ.** Ce qui le distingue est la capacité à expliquer le pas en cours : valeurs avant/après, branche qui sera prise, adresse effective, conventions et erreurs. Je développerais cette qualité en priorité.

Cette revue porte sur l’arbre de travail présent, version déclarée 0.2.9-beta.2, qui contenait déjà de nombreuses modifications. Elle associe lecture ciblée du code, exécution de la suite de tests et inspection de captures. Ce n’est ni une lecture exhaustive des quelque 152 000 lignes Rust (tests, commentaires et exemples inclus), ni une certification de justesse du processeur simulé. Aucun fichier existant de l’application n’a été modifié pendant la revue.

## Ce qui est convaincant

- **Une intention produit précise.** L’explication de l’exécution apporte une valeur plus nette qu’une accumulation de panneaux d’analyse. Le retour en arrière et les exercices renforcent cette intention.
- **Un découpage métier utile.** Onze crates séparent notamment mémoire et registres, assemblage, décodage, émulation, analyse et interface. Utiliser NASM et iced-x86 évite de réinventer deux fondations difficiles.
- **Des tests qui cherchent des erreurs réelles.** La suite vérifie les états machine, les entrées ELF malformées, les parcours et la géométrie de l’interface. L’exemple `namsdec-emu/examples/oracle.rs` prévoit aussi une comparaison avec une exécution de référence ; il n’a pas été exécuté dans cette revue.
- **Plusieurs précautions de robustesse sont déjà concrètes.** Le lanceur de décompilateurs borne les sorties, impose une échéance et traite les groupes de processus. Le chargeur et les analyseurs ont des contrôles sur les entrées hostiles. Ces points méritent d’être conservés et étendus.

## Ce qui me retient

### 1. La fidélité de l’émulation doit être annoncée plus précisément

Dans `crates/namsdec-emu/src/x87.rs:296`, `Fldcw` est accepté sans changer le comportement ; le commentaire précise que le calcul reste en double précision avec un seul mode d’arrondi. Les conversions d’opérandes 80 bits passent par `f64` (`x87.rs:41` et `:336`).

**Conséquence :** certains programmes peuvent continuer avec un résultat différent de ce que leur réglage du processeur demande. Pour un outil pédagogique, une approximation silencieuse peut enseigner un comportement faux. Je distinguerais explicitement « fidèle », « simplifié » et « non pris en charge », avec un avertissement à l’exécution pour les cas concernés. C’est un constat de code, pas un échec reproduit de la suite actuelle.

### 2. L’assemblage peut bloquer l’interface

`crates/namsdec-gui/src/run.rs:44` appelle directement l’assembleur. Dans `crates/namsdec-asm/src/lib.rs:306`, NASM puis ld sont attendus avec `Command::output()` (`:329`, `:345`), sans délai visible sur ce chemin.

**Conséquence :** une source très coûteuse ou un outil qui ne répond plus peut immobiliser le fil de l’interface. Le projet sait déjà exécuter des travaux en arrière-plan ; j’appliquerais le même principe à l’assemblage avec une annulation et une limite de durée. Le blocage prolongé n’a pas été provoqué pendant cette revue.

### 3. L’annulation n’a pas les mêmes garanties partout

L’installation d’outils dans `crates/namsdec-gui/src/jobs.rs:85` tue l’enfant direct. À l’inverse, `crates/namsdec-disasm/src/plugin.rs` prévoit un groupe de processus et une récupération des sorties avec délai de grâce. La voie d’installation joint également les lecteurs sans échéance (`jobs.rs:106`).

**Conséquence possible :** annuler une compilation ne garantit pas l’arrêt de ses descendants ; un descendant qui conserve un tube ouvert peut prolonger l’attente. Je mutualiserais le lanceur robuste au lieu de maintenir ces garanties séparément. Il s’agit ici d’un risque identifié dans le code, sans reproduction d’une fuite de processus.

### 4. L’état de l’interface est trop centralisé

`NamsdecApp` commence dans `crates/namsdec-gui/src/main.rs:589` et occupe près de mille lignes, commentaires compris. La séparation en fichiers améliore la navigation, mais beaucoup de ces fichiers étendent encore la même structure et partagent son état.

**Conséquence :** ajouter une fonction peut toucher beaucoup d’invariants implicites. Je regrouperais progressivement les états par responsabilité : document, session d’exécution, espace d’analyse, travaux de fond et présentation. Une réécriture générale n’est pas justifiée par cette revue.

### 5. La présentation demande trop de familiarité au premier regard

Sur la capture fraîche de la vue source chargée, les nombreuses vues, les commandes d’exécution et l’inspecteur ont une importance visuelle proche. Le source prend un grand espace relativement vide tandis que l’explication pédagogique, pourtant distinctive, reste dans une colonne étroite. La capture d’analyse montre une longue navigation de modules dans une fenêtre superposée à l’atelier.

Ce ne sont pas des défauts de géométrie démontrés : les contrôles automatiques passent. C’est un jugement de hiérarchie et de charge de lecture. Je chercherais à rendre plus évident le chemin « lire → prévoir → avancer → constater », puis à proposer un espace distinct pour l’analyse avancée. Les trois maquettes matérialisent cette proposition. Une validation auprès de débutants et d’utilisateurs expérimentés reste nécessaire.

### 6. Les commentaires gardent trop souvent le journal des corrections

Beaucoup expliquent utilement un invariant. D’autres racontent sur plusieurs paragraphes un ancien défaut, une tentative, puis sa réfutation. Cette mémoire est précieuse, mais elle finit par masquer l’état actuel du contrat. Je garderais dans le code les raisons durables et déplacerais une partie du récit historique vers des notes techniques.

## Vérifications réellement exécutées

- `cargo test --workspace --offline` : **1 405 réussis, 0 échec, 5 ignorés**, résultats agrégés des tests unitaires, d’intégration et de documentation.
- `cargo test -p namsdec-gui --offline captures_de_toutes_les_scenes -- --ignored --nocapture` : **réussi, 112 images produites** dans `target/captures/`.
- Inspection visuelle ciblée des captures `vue___source_(en_marche).png` et `analyse_(en_marche).png` ; les 112 images n’ont pas toutes été examinées individuellement.
- Ni clippy, ni l’oracle matériel, ni un audit externe des dépendances n’ont été exécutés dans cette revue. Aucun jugement sur leur résultat ne doit être déduit des tests passés.

## Ordre de travail que je choisirais

1. Rendre visibles les limites de simulation et renforcer la comparaison avec une référence indépendante.
2. Garantir une interface réactive et une annulation cohérente pour tous les outils externes.
3. Séparer clairement l’expérience d’apprentissage de l’analyse avancée.
4. Extraire les sous-états de l’interface au fil des changements nécessaires.

**Je le considère comme une bêta crédible et ambitieuse.** Sa suite de tests inspire confiance sur les comportements couverts ; elle ne suffit pas à le présenter comme un émulateur universel ou un analyseur exhaustif. Le meilleur investissement me semble être la consolidation et la clarté, avant un nouveau lot de modules.
