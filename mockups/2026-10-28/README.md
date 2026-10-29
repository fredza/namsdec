# Trois propositions pour Namsdec

Ouvrir `index.html` dans un navigateur. Les fichiers fonctionnent directement depuis le disque, sans serveur, compilation, téléchargement, police externe ou installation.

| Fichier | Direction | Compromis |
| --- | --- | --- |
| `atelier.html` | Atelier guidé, clair et pédagogique | Moins de données simultanées ; priorité à l’explication |
| `observatoire.html` | Poste d’analyse sombre, trois colonnes | Plus dense ; adapté à un utilisateur qui connaît déjà ses outils |
| `trajectoire.html` | Graphe central et navigation temporelle | Très lisible pour les branches ; le source complet demanderait une vue complémentaire |

Il s’agit de trois propositions d’organisation, pas de trois palettes appliquées au même écran. Elles étudient un écran représentatif de l’application, pas la totalité de ses modules et dialogues. Les styles et le comportement commun vivent dans `styles.css` et `app.js`.

## Ce qui fonctionne

- Avancer, reculer, exécuter automatiquement, mettre en pause et repartir.
- Raccourcis F10 et Maj + F10.
- Basculer entre source, désassemblage et analyse dans les deux premières propositions.
- Consulter les registres, les octets et l’explication de la pile.
- Passer entre console, trace et diagnostics.
- Consulter l’aide de l’instruction courante.
- Parcourir les 21 états de la démonstration avec la chronologie de Trajectoire.

L’exemple additionne les entiers de 1 à 5, puis renvoie 15 comme code de sortie Linux. Il n’imprime rien. Le même scénario alimente les trois propositions, initialement arrêtées au pas 4, avant le saut conditionnel. La simulation est volontairement bornée à cet exemple ; aucun assembleur, émulateur natif, fichier utilisateur ou service distant n’est appelé. Le source affiché est en lecture seule. Les listes de cours et de symboles sont des repères visuels, pas des actions non implémentées.

## Transposition possible en egui

Atelier : panneau latéral de parcours, espace source central, explication persistante à droite. Observatoire : panneaux symboles / listing / inspecteur. Trajectoire : espace de dessin pour le graphe, inspecteur et bande temporelle inférieure. Les commandes métier existantes restent les points d’entrée ; la conception ne demande pas de remplacer le moteur par une technologie web.

Ma préférence : Atelier par défaut, Observatoire comme espace avancé, Trajectoire comme vue du programme. Éviter de faire apparaître tous les modules au premier lancement.

## Périmètre de vérification

Voir `ANALYSE.md` pour les résultats sur l’application Rust. Les maquettes ont fait l’objet d’une vérification de syntaxe et de leur scénario interactif hors navigateur. Aucun navigateur pilotable n’était disponible dans cette session : la mise en page HTML n’a pas été validée dans un navigateur réel.
