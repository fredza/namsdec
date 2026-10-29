# Namsdec RE — maquette de poste d’analyse

Ouvrir [re.html](re.html) directement dans le navigateur ou depuis [la galerie](../index.html).

Cette maquette explore un espace de travail dédié au reverse engineering. Elle n’est pas une version web fonctionnelle de Namsdec. Les instructions, chaînes, protections et le pseudo-code sont des données fictives écrites pour l’interface, sans analyse de fichier ni appel à un moteur.

## Parcours interactif

1. Ouvrir Signaux et rejoindre la comparaison des premiers octets de l’entrée.
2. Sélectionner une instruction : ses octets, son explication et ses références apparaissent dans l’inspecteur. Le pseudo-code correspondant est surligné.
3. Suivre une cible avec la flèche du listing ; revenir avec la flèche de navigation.
4. Parcourir le graphe de main ou de verify_token et cliquer un bloc pour retrouver le code.
5. Rejoindre les instructions qui référencent une chaîne ou l’import puts.
6. Renommer une fonction avec un identifiant ASCII ; son nom se retrouve dans l’explorateur, le listing et le pseudo-code.
7. Annoter une adresse. Les notes restent en mémoire dans cette page et se consultent dans Notes. « Exporter les notes » télécharge un JSON comprenant les notes et les noms.

Les notes ne survivent pas au rechargement : exporter le JSON avant de fermer la page. Aucun import de notes n’est implémenté.

Le scénario montre une vérification de préfixe ASCII N puis D, avec deux messages de sortie. Le pseudo-code est une illustration manuelle, pas une sortie de DZF ou d’un autre outil. Les statuts des protections sont eux aussi fictifs et signalés comme tels. Les zones de code sont en lecture seule.

## Une vraie version web locale : architecture proposée

Le navigateur fournirait la présentation ; un processus Rust lancé par l’utilisateur hébergerait l’interface et les opérations sur sa machine.

```text
Navigateur
    │ API locale : chargement, navigation, commandes, résultats
    ▼
Service Namsdec lié à 127.0.0.1
    ├── namsdec-core : état du programme et mémoire
    ├── namsdec-disasm : désassemblage
    ├── namsdec-emu : exécution et historique
    ├── namsdec-re : analyses
    └── outils locaux : NASM, ld, DZF, décompilateurs configurés
```

Le service pourrait servir lui-même les fichiers de l’interface : une seule application à lancer, puis une adresse locale à ouvrir. Aucun compte, cloud ou envoi de binaire vers un service distant ne serait nécessaire pour ces opérations locales. Une éventuelle recherche de mise à jour ou installation d’outils resterait une opération réseau explicite.

Le code actuel offre des bases réutilisables, mais le travail n’est pas un simple changement de rendu : `namsdec-re` dépend d’egui et certains modules réunissent calcul et affichage. Il faudrait extraire des résultats d’analyse structurés, rendre les travaux longs asynchrones et présenter leurs résultats dans le navigateur. Le chargement des fichiers, les sessions, l’exécution, les outils externes et les modifications de binaires demanderaient chacun leur raccordement.

Le service devrait limiter ses appels au navigateur local autorisé et exposer des opérations métier précises. Lancer des outils natifs ou modifier un fichier resterait une opération du service Rust ; le JavaScript de la page ne recevrait pas un accès arbitraire à la machine.

## Périmètre d’un premier portage utile

| Fonction | Maquette actuelle | Version locale à développer |
| --- | --- | --- |
| Ouvrir un ELF | Scénario fictif préchargé | Charger et analyser un fichier local |
| Fonctions et instructions | Données écrites à la main | Résultats des moteurs Rust |
| Chaînes et références | Navigation sur deux chaînes | Extraction et index des références |
| Graphe | Deux graphes prédéfinis | CFG calculé et navigation dans les blocs |
| Pseudo-code | Illustration manuelle | Sortie de DZF ou d’un outil configuré, avec progression et erreurs |
| Notes et noms | Mémoire de la page, export JSON | Projet local enregistré et rechargeable |
| Émulation, retour arrière | Non représentés dans cette vue RE | Commandes reliées au moteur existant |
| Correctifs, hexadécimal, détours, autres modules | Non représentés | Portage progressif des opérations existantes |

## Vérification

Syntaxe JavaScript, navigation entre vues, cibles de références, renommage, notes et export vérifiés par scripts avec un DOM simulé. Les fichiers de données sont cohérents sur les longueurs d’instructions et les déplacements relatifs représentés. Liens locaux et structure CSS vérifiés.

Le rendu visuel et les interactions du navigateur restent à contrôler dans un navigateur réel, indisponible au pilotage dans cette session. Aucun moteur Rust n’a été lancé pour la maquette.
