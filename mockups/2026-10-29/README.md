# Cinq nouvelles maquettes pour Namsdec

Ouvrir [la galerie des huit propositions](../index.html) dans un navigateur. Tout fonctionne depuis le disque, sans serveur, dépendance, téléchargement ni police distante.

| Proposition | Organisation | Interaction particulière |
| --- | --- | --- |
| [Focus](focus.html) | Éditeur sombre, inspecteur escamotable, commandes en bas | Afficher ou masquer l’explication ; basculer source / désassemblage |
| [Carnet](carnet.html) | Carnet clair avec marge de chapitre, source et question | Prévoir le résultat de JNZ et obtenir une explication |
| [Delta](delta.html) | Référence et état courant côte à côte | Épingler un état, parcourir les pas, lire les différences |
| [Prisme](prisme.html) | Image binaire, listing avec octets, inspecteur | Sélectionner une instruction ou suivre RIP ; retrouver ses octets |
| [Namsdec RE](re.html) | Poste d’analyse avec code, pseudo-code et inspecteur | Références croisées, graphe, chaînes, noms et notes ; voir [le périmètre RE](README-RE.md) |

## Scénario

Focus, Carnet, Delta et Prisme utilisent le même exemple que les maquettes du 28 octobre : la somme des entiers de 1 à 5. Il comporte 20 instructions exécutées, soit 21 états avec l’état initial. Le programme termine par `exit(15)` et n’affiche aucun texte. Focus, Carnet et Prisme s’ouvrent au pas 4 ; Delta s’ouvre au pas 6 avec le pas 3 épinglé pour rendre immédiatement visibles les écarts. Namsdec RE utilise un autre scénario fictif : une vérification de préfixe ASCII.

Avancer, reculer, repartir et exécuter automatiquement fonctionnent dans les quatre vues. F10 avance ; Maj + F10 recule. L’exécution automatique s’arrête à la fin, lors d’une navigation manuelle dans le temps, ou lorsque la page est masquée. Le curseur de Delta applique la sélection lorsqu’on le relâche.

Prisme distingue l’instruction sélectionnée de l’instruction en cours : sélectionner une ligne suspend le suivi de RIP sans déplacer l’exécution. « Reprendre le suivi RIP » rétablit la liaison. Les valeurs de registres restent celles du pas courant.

Il s’agit de démonstrations à scénario fixe. Le source est en lecture seule ; aucun fichier utilisateur, assembleur, émulateur natif ou outil externe n’est lancé. La marge de chapitre de Carnet et les métadonnées de l’image de Prisme sont des repères, pas des commandes. Seuls RAX, RCX, RDI, RIP et quelques indicateurs utiles à cet exemple sont représentés. Aucun de ces écrans n’est intégré à l’application Rust.

## Transposition en egui

- Focus : panneau central et panneau droit facultatif ; commandes hors de la zone de défilement du listing.
- Carnet : source, explication et question côte à côte, puis empilés lorsque la largeur manque.
- Delta : capture de référence indépendante du curseur de l’historique ; différences calculées contre cette référence.
- Prisme : adresse de sélection distincte de RIP ; état explicite du suivi automatique.

Les commandes existantes de Namsdec peuvent alimenter ces vues. Les maquettes ne proposent pas de remplacer le moteur ou le rendu Rust par une application web.

## Vérification

Syntaxe JavaScript, états du scénario, parcours des interactions et liens locaux vérifiés par scripts. Les contrôles couvrent le retour arrière, les bornes, la fin de l’exécution automatique, le quiz, l’épinglage, la chronologie et la sélection des octets. Le contrôle de structure HTML recherche aussi les identifiants dupliqués.

Aucun navigateur pilotable n’est disponible dans cette session : le rendu visuel, le comportement réel du focus et les dispositions responsives restent à vérifier dans un navigateur. Les points de rupture CSS prévoient une disposition empilée sur écran étroit et un défilement local pour les listings. Aucun test Rust n’a été relancé pour ces fichiers de conception.
