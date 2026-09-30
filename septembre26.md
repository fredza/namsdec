# namsdec — analyse, corrections et améliorations

*Relevé du 1er octobre 2026, version 0.2.6-beta.1. namsdec est un atelier
pédagogique d'assembleur x86-64 et de rétro-ingénierie : il aide un
chercheur à **lire, comprendre et analyser** un binaire — y compris du
code malveillant — en simulant le processeur plutôt qu'en l'exécutant.*

Ce fichier double `TRAVAUX.md` (l'état) et `AMELIORATIONS.txt` (le
pourquoi) d'une lecture transversale, centrée sur ce qu'un chercheur en
sécurité attend de l'outil. Chaque point dit ce qu'il apporte, ce qu'il
coûte, et où il se branche dans le code existant.

---

## État des lieux

Ce qui est solide, et sur quoi le reste s'appuie :

- **On n'exécute jamais la cible sur la machine hôte.** Le binaire tourne
  dans un émulateur (`namsdec-emu`), pas sur le vrai processeur. C'est la
  bonne fondation pour analyser du code hostile : rien de ce qu'il
  contient ne s'exécute pour de vrai.
- **Les analyseurs supposent un fichier hostile.** `namsdec-re/elf_scan`
  le dit en toutes lettres et lit tout par `get`/`Option`, jamais par
  indexation nue.
- **Plus de mille tests**, CI verte (mise en forme, clippy sans
  indulgence, `cargo test --workspace`), coureur épinglé à `ubuntu-24.04`.
- **Dix panneaux d'analyse** déjà en place : empreintes, carte du fichier,
  protections, entropie, fonctions, chaînes, figures du code, appels
  système, imports, fragments ROP, place libre, vidage.

---

## Corrections

### C1 — [FAIT le 1er octobre] Panique sur ELF hostile dans `imports::scan`

Un fuzzing léger (40 000 ELF malformés) a trouvé **deux paniques**, toutes
deux dans `imports::scan` : `crates/namsdec-re/src/imports.rs:185`,
`sym * sym_entry` où `sym` vient d'un champ du fichier (`info >> 32`),
quelconque sur une entrée hostile. En debug, le débordement panique ; en
release, il aurait donné un index faux.

**C'est la première exigence d'un outil d'analyse de malware** : les
fichiers reçus sont délibérément malformés, et un analyseur qui tombe
dessus n'analyse plus rien. Corrigé par arithmétique vérifiée
(`checked_mul`/`checked_add`), comme le reste du module le faisait déjà
plus haut. L'invariant est désormais tenu par un test :
`a_hostile_elf_never_panics_the_scan`. Reconfirmé : 0 panique sur 40 000.

### C2 — Étendre l'invariant « aucune panique sur entrée hostile » à TOUS les analyseurs

Le test ci-dessus couvre `imports`, `slack` et `elf_scan`. Restent à y
passer, avec le même fuzzing : `functions`, `xrefs`, `strings`,
`patterns`, `gadgets`, `entropy`, `protections`, `syscalls`, et surtout le
**désassembleur** (`namsdec-disasm`), qui reçoit des octets arbitraires
par construction. Coût faible (un générateur reproductible, déjà écrit),
valeur haute : c'est le contrat de robustesse de tout l'outil.

### C3 — Les seuils heuristiques, rassemblés et documentés

`entropy.rs` tranche « compressé/chiffré/empaqueté » à 7,2 bits, « code
dense » à 6,5. Ces nombres sont justes mais dispersés et implicites. Les
réunir en constantes nommées, avec la mesure qui les justifie, évite
qu'un chercheur les prenne pour des vérités plutôt que pour des repères —
et permet de les régler sans relire l'algorithme.

---

## Améliorations — analyse (la plus forte valeur pour la vocation)

### A1 — Catégoriser les imports et les appels système par COMPORTEMENT

Aujourd'hui les imports et les syscalls sont **listés**. Pour un
chercheur, la question n'est pas « quelles fonctions » mais « qu'est-ce
que ce binaire *fait* » : le regrouper par famille répond d'un coup d'œil.

- **Réseau** : `socket`, `connect`, `send`, `recv`, `bind`, `sys_socket`…
- **Exécution** : `execve`, `system`, `fork`, `clone`, `posix_spawn`…
- **Anti-débogage / évasion** : `ptrace`, `sys_ptrace`, lecture de
  `/proc/self/status`, `prctl`.
- **Chiffrement / obfuscation** : familles crypto, `mprotect` vers RWX.
- **Fichiers / persistance** : `open`, `unlink`, chemins système.

Se branche sur `imports.rs` et `syscalls.rs` (une table nom → famille) et
se rend dans les panneaux existants, en colorant ou en groupant. C'est
l'ajout qui transforme le plus l'outil pour la détection : trois familles
allumées — réseau, exécution, anti-débogage — est déjà un portrait.

### A2 — Un panneau « Signaux » : le verdict d'un coup d'œil

Un panneau de synthèse qui agrège ce que les autres trouvent déjà, chacun
avec sa raison, sans jamais conclure à la place du chercheur :

- segment **RWX** (inscriptible ET exécutable) — `Scan::writable_executable`
  existe déjà ;
- **entropie élevée** en fin de fichier → probablement empaqueté —
  `entropy.rs` le sait déjà, il suffit de le remonter ;
- **`ptrace`** présent → anti-débogage (via A1) ;
- protections **absentes** (NX, canari, RELRO, PIE) — `protections.rs` ;
- **code auto-modifiant** possible (`mprotect`/`mmap` + saut dans la zone).

Chaque ligne renvoie au panneau qui l'explique. C'est le tableau de bord
qu'un analyste lit en premier, et il ne coûte que du câblage : toute la
matière est déjà calculée ailleurs.

### A3 — Extraire les indicateurs (IoC) des chaînes

`strings.rs` liste les chaînes ; il ne les **qualifie** pas. Un module (ou
une colonne) qui repère les URL, adresses IP, chemins de fichiers, clés de
registre et noms de domaines donne au chercheur les indicateurs qu'il
recopiera dans son rapport. Purement lecture, et très rentable : c'est la
première chose qu'on cherche dans un échantillon.

### A4 — Détection d'empaqueteur nommée

`entropy` dit « probablement empaqueté » ; aller un cran plus loin en
reconnaissant les signatures les plus courantes (UPX et consorts : nom de
section, en-tête) nomme l'empaqueteur au lieu de le deviner. Borné,
lecture seule, et complète bien A2.

### A5 — Le désassemblage suit le flot, pas seulement la linéarité

Vérifier (le cas échéant, améliorer) que le désassemblage gère le
**désalignement volontaire** — l'anti-désassemblage classique : un saut
qui atterrit au milieu d'une instruction pour tromper la lecture linéaire.
Un outil d'analyse de malware rencontre cette figure ; la montrer (« ici
le flot lit d'autres octets que la lecture ligne à ligne ») est
pédagogique *et* utile.

---

## Améliorations — le gros œuvre déjà identifié

### B1 — Virtualiser le listing (A 5.3, mesuré)

Premier tracé d'un gros binaire : 325 ms pour 15 000 lignes, dont **285 de
pose des glyphes** (87 %). Payé une fois par chargement, le cache fait le
reste (2,4 ms en régime). Le seul vrai remède : ne poser que les lignes
visibles — gain ~×300 sur la première image, coût d'une vue en lecture
seule pour les listings (sélection, recherche, curseur, marques à
reporter). Gros chantier, à décider en connaissance de cause. Mesures
complètes dans `AMELIORATIONS.txt` §5.3.

### B2 — Modifier le binaire ouvert (onglet RE)

Premier morceau fait : le panneau **Place libre** montre où l'éditeur de
liens a laissé du vide, nommé par section. La suite — assembler dans une
zone libre puis y router l'exécution et la ramener — est à construire
**pas à pas, chaque geste réversible et montré avant d'être posé**, via
`patch.rs` (qui sait déjà défaire) et `ReRequest::PatchBytes` (qui
existe). Cadre pédagogique explicite, comme le panneau des fragments ROP.
Voir la mémoire `project-namsdec-modifier-binaire`.

### B3 — Le multicurseur (A 4.14)

Bloqué par egui, qui ne donne pas la main sur plusieurs curseurs dans un
`TextEdit`. À reprendre si egui évolue, ou via un champ maison — ce qui
rejoint B1 (une vue de listing qu'on ne délègue plus entièrement à egui).

---

## Améliorations — ergonomie et finition

- **Naviguer depuis les panneaux d'analyse vers le listing** : cliquer une
  zone libre, un import, une figure, un fragment devrait sauter à son
  adresse dans le désassemblage. Demande un canal de navigation
  (`ReRequest` ou équivalent) ; petit et très utile pour circuler dans un
  échantillon.
- **Exporter un rapport d'analyse** : `report.rs` existe ; un bouton « tout
  exporter » (empreintes + signaux + IoC + imports catégorisés) donnerait
  au chercheur le compte rendu prêt à joindre.
- **Les raccourcis repris par le sens** (A 7.1) : le jeu Vim force la main
  (`n` court-circuite `Espace n i`). Refonte proposée de longue date.

---

## Priorisation suggérée

1. **C2** — finir l'invariant anti-panique sur tous les analyseurs et le
   désassembleur. C'est le socle : un outil de malware doit encaisser
   n'importe quel fichier. Petit, immédiat.
2. **A1 + A2** — catégoriser par comportement, puis le panneau Signaux.
   C'est ce qui transforme le plus l'outil pour sa vocation, et tout
   s'appuie sur du déjà-calculé.
3. **A3** — les IoC dans les chaînes. Rentable, borné, lecture seule.
4. **B1** — la virtualisation, quand le confort sur gros binaires le
   justifie. Gros, à décider.
5. **B2** — la modification de binaire, par petits pas réversibles.

Rien de ce qui précède ne casse l'existant : les corrections sont faites
et testées, les améliorations se branchent sur des données déjà produites,
et le gros œuvre est chiffré avant d'être entrepris.
