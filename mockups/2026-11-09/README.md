# Trois modes simples — 9 novembre 2026

> **Décidé le 9 novembre 2026 : l'Établi, et la Scène gardée en bouton.**
> Les deux sont faits dans l'application — `crates/namsdec-gui/src/etabli.rs`
> et `crates/namsdec-gui/src/scene.rs`. Le Carnet n'est pas retenu ; sa
> question de prévision existe déjà dans l'Établi, sous le code
> (`prevision.rs`). Ces pages restent ce qu'elles sont : l'état de la
> réflexion au moment du choix.

Ouvrir [la galerie](index.html) dans un navigateur. Tout fonctionne depuis le disque : aucun serveur, aucune dépendance, aucune police distante, aucun réseau.

| Proposition | Organisation | Le pari | Ce qu’il coûte |
| --- | --- | --- | --- |
| [L’Établi](etabli.html) | Quatre bandes fixes : code, machine, ligne courante, résultat | Tout tient à l’écran en même temps, donc rien n’est à chercher | La place est répartie d’avance et ne se redistribue pas |
| [Le Carnet](carnet.html) | Une colonne, l’état écrit en marge de la ligne courante | Qui apprend lit un texte, pas un tableau de bord | On ne voit qu’un instant à la fois |
| [La Scène](scene.html) | Deux temps séparés : « Écrire », puis « Regarder » | Écrire et regarder sont deux moments, les mêler les brouille | Corriger demande de revenir au premier temps |

## Le scénario

Les trois utilisent le même programme : **la somme de 1 à 5, calculée dans une fonction appelée par `call`**. Dix lignes d’instructions, **22 instructions exécutées**, 23 états avec l’état initial, et `exit(15)` pour finir.

Deux instructions de plus que l’exemple du 29 octobre (`call` et `ret`), et pour une raison : sans elles, la pile ne bouge jamais, et un endroit où regarder qui ne bouge pas n’apprend rien. Le programme n’affiche rien — c’est voulu, et chaque maquette le dit : un premier programme en assembleur calcule, il ne parle pas encore. La Scène va plus loin et lève le piège de la fin : **RAX vaut 60, pas 15**, parce qu’il porte le numéro de `exit` ; la somme est passée dans RDI.

Les trois s’ouvrent au **pas 4** : la boucle a déjà tourné une fois, donc la pile porte une case et une valeur vient de changer. Un écran vide ne se juge pas.

## Ce qui marche dans les trois

- avancer, revenir, tout dérouler, reprendre — au bouton comme au clavier (F10, Maj+F10, Espace, Début) ;
- l’exécution automatique s’arrête d’elle-même à la fin, à toute navigation manuelle, et quand l’onglet passe en arrière-plan ;
- **l’écran d’une faute de frappe** : le lien en bas de chaque maquette met `add rax, ecx` à la place de `add rax, rcx` et montre ce que chacune en fait — ligne marquée, message de nasm en clair, explication, bouton qui corrige. Un écran se juge aussi à ce qu’il fait quand ça rate ;
- `?pas=N` ouvre n’importe quel instant (0 à 22), `?erreur=1` ouvre l’écran de la faute. C’est ce qui rend une capture reproductible et un contrôle écrivable.

Particularités : l’Établi pose des points d’arrêt dans la gouttière et ouvre la fiche de l’instruction sous l’explication ; le Carnet replie l’en-tête du fichier et pose la question de prévision ; la Scène recentre son ruban sur l’instruction courante et distingue ce qui a déjà tourné de ce qui n’a jamais tourné.

## La prévision, dans le Carnet

Elle suit la règle déjà retenue dans l’application (`crates/namsdec-gui/src/prevision.rs`) : on répond **avant**, le verdict n’arrive qu’**après** le pas, et il **ne donne jamais la bonne réponse** — on l’obtient en exécutant, ce que l’exercice enseigne. Le choix retenu est marqué comme *choisi*, jamais comme juste : rien à l’écran ne doit laisser deviner avant le pas. Répondre n’est jamais obligatoire.

Deux détails qui comptent plus qu’il n’y paraît. La bonne réponse est écrite en premier dans le code, parce que c’est ainsi qu’on relit ces questions — mais elle est **proposée** à une place qui dépend du pas : sinon on apprend à cliquer à gauche, pas à lire le programme. Et la rotation est déterministe, donc une capture reste comparable et un contrôle sait d’avance où est la réponse.

## Transposition en egui

- **L’Établi** : `Panel::top` pour la barre et la frise, `Panel::right` pour la machine, deux `Panel::bottom` pour l’explication et le résultat, `CentralPanel` pour le listing. C’est la disposition que l’application sait déjà faire ; le travail serait surtout de *retirer* (rails, onglets de vues, barre d’état, menus) et d’ajouter la bande d’explication.
- **Le Carnet** : une colonne centrée de largeur bornée, et la note en marge posée à la hauteur de la ligne courante. Le point délicat est le même qu’ici : **la note ne doit pas écarter les lignes voisines** — sinon le listing saute d’une trentaine de pixels à chaque pas. En egui, peindre la note dans la gouttière droite sans l’inclure dans la hauteur de la rangée.
- **La Scène** : deux états exclusifs de la zone centrale, et un ruban dont le décalage est calculé pour centrer l’instruction courante. Le recentrage doit se refaire au redimensionnement.
- Dans les trois : `Niveau::Débutant` ne filtre plus des panneaux, il **choisit une disposition**. C’est le vrai changement par rapport à aujourd’hui, où le mode débutant est l’interface complète avec des morceaux en moins.

## Vérification

Faite dans cette séance, avec Chromium sans fenêtre (`chromium-browser --headless`) :

- syntaxe JavaScript des cinq fichiers (`node --check`) ;
- cohérence du scénario : 23 états, somme 15, pile revenue à vide, RSP revenu à sa valeur de départ ;
- **aucune exception JavaScript** sur 27 états : les trois maquettes aux pas 0, 1, 3, 4, 12, 18, 19 et 22, plus l’écran de la faute ;
- captures des trois maquettes aux pas 4 et 22, et de l’écran de la faute ;
- **deux rendus successifs identiques au pixel près**, sur les trois — ce qui a demandé de couper les transitions sous `prefers-reduced-motion` (`base.css`). Une capture prise pendant un fondu de 120 ms a fait croire pendant dix minutes qu’un bouton éteint gardait son fond bleu : il était simplement en train de le perdre.

Trois défauts corrigés, dont deux que seule la mesure a montrés : le bouton éteint ci-dessus, et le pupitre du Carnet qui, flottant, **mangeait la dernière réponse de la question** — il est passé dans le flux, collé en bas, et porte maintenant l’état de la machine. Le troisième vient de la relecture : la bonne réponse était proposée en premier dans sept questions sur huit.

Non vérifié : le comportement réel du focus au clavier, la lecture par un lecteur d’écran, et le rendu sous un autre navigateur. Aucun test Rust n’a été relancé — aucune ligne de l’application n’est touchée par ces fichiers.
