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
- **Onze panneaux d'analyse** en place depuis le 1er octobre : empreintes, carte du fichier,
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

### C2 — [FAIT le 1er octobre] L'invariant étendu à TOUS les analyseurs

Fait : `crates/namsdec-re/tests/hostile.rs` fait passer une vingtaine
d'analyseurs, le **désassembleur** et les deux rapports sur des ELF abîmés
de trois façons — octets changés, troncature, et valeurs extrêmes posées
sur huit octets alignés. Le germe est **fabriqué** (en-tête, segments,
sections, symboles, relocations, entrées de liaison) : le contrôle
d'origine partait de `/bin/true` et passait sans rien prouver sur une
machine qui n'en a pas.

**Quatre paniques trouvées** : deux sommes d'adresses dans
`imports::file_offset`, deux dans `slack`, et surtout les trois lecteurs
`u16_at`/`u32_at`/`u64_at` d'`elf_scan` et d'`imports` — `at + 4` déborde
dès que `at` vient d'un champ du fichier. Ce n'était pas un défaut mais
une **classe** de défauts : les quarante additions de décalage de champ
sont désormais saturantes.

### C3 — [FAIT le 1er octobre] Les seuils heuristiques, rassemblés et documentés

Fait : `entropy::seuils` porte les quatre bornes, chacune avec la mesure
qui la justifie, et le panneau les lit là plutôt que de les recopier. Un
contrôle vérifie qu'elles se suivent — un seuil « plat » posé au-dessus
du seuil « texte » rendrait une branche du `match` inatteignable sans que
rien ne le dise.

---

## Améliorations — analyse (la plus forte valeur pour la vocation)

### A1 — [FAIT le 1er octobre] Catégoriser les imports et les appels système par COMPORTEMENT

Aujourd'hui les imports et les syscalls sont **listés**. Pour un
chercheur, la question n'est pas « quelles fonctions » mais « qu'est-ce
que ce binaire *fait* » : le regrouper par famille répond d'un coup d'œil.

- **Réseau** : `socket`, `connect`, `send`, `recv`, `bind`, `sys_socket`…
- **Exécution** : `execve`, `system`, `fork`, `clone`, `posix_spawn`…
- **Anti-débogage / évasion** : `ptrace`, `sys_ptrace`, lecture de
  `/proc/self/status`, `prctl`.
- **Chiffrement / obfuscation** : familles crypto, `mprotect` vers RWX.
- **Fichiers / persistance** : `open`, `unlink`, chemins système.

Fait : `behaviour.rs`, six familles, **une seule table** pour les imports
et les appels système — un `socket` est un `socket` qu'il vienne de la
table de liaison ou du numéro 41 dans RAX. Le portrait se dessine en tête
des deux panneaux, avec les mêmes couleurs.

Une table de noms exacts et non des règles : « tout ce qui commence par
`exec` » attraperait `execute_query`. Et quatre noms ont été **retirés**
de l'anti-débogage après coup — `raise`, `alarm`, `sigaltstack`,
`rt_sigaction` : une capture d'écran a montré `/bin/ls` allumant une
ligne rouge sur `raise`, et un tableau de bord qui crie sur `ls` n'est
plus lu du tout.

### A2 — [FAIT le 1er octobre] Un panneau « Signaux » : ce qui ressort, et où le lire

Un panneau de synthèse qui agrège ce que les autres trouvent déjà, chacun
avec sa raison, sans jamais conclure à la place du chercheur :

- segment **RWX** (inscriptible ET exécutable) — `Scan::writable_executable`
  existe déjà ;
- **entropie élevée** en fin de fichier → probablement empaqueté —
  `entropy.rs` le sait déjà, il suffit de le remonter ;
- **`ptrace`** présent → anti-débogage (via A1) ;
- protections **absentes** (NX, canari, RELRO, PIE) — `protections.rs` ;
- **code auto-modifiant** possible (`mprotect`/`mmap` + saut dans la zone).

Fait : `signals.rs`, en seconde position dans la colonne. Chaque ligne dit
un **fait** vérifiable, **ce qu'il change**, et **le panneau** où aller
voir. **Rien n'est additionné en score** : un score donnerait l'illusion
d'un verdict, et c'est le chercheur qui conclut.

Deux garde-fous, et tous deux ont servi : sur `/bin/true` et `/bin/ls`,
aucune ligne « Remarquable » ; et l'entropie n'est relevée que sur une
tranche d'au moins 4 Kio — la queue de `/bin/true` fait 505 octets et
mesure 7,61 sans rien cacher, simplement parce qu'il n'y a pas de quoi
répéter un octet dans cinq cents.

### A3 — [FAIT le 1er octobre] Extraire les indicateurs (IoC) des chaînes

Fait : `ioc.rs` — adresses web, adresses IP, noms de domaine, courriels,
chemins absolus, clés de registre, en tête du panneau des chaînes, chaque
rubrique copiable d'un clic. La moitié du travail est de **ne pas**
relever : une liste où `libc.so.6` passe pour un nom de domaine ne se lit
pas, et l'on cesse de l'ouvrir. D'où une liste de domaines de premier
niveau connus, et des chemins absolus d'au moins deux segments.

Une ambiguïté reste, et n'est pas levée : `1.2.3.4` est un numéro de
version pour l'un et une adresse pour l'autre. On relève, et le panneau
dit « indicateur », non « adresse contactée ».

### A4 — [FAIT le 1er octobre] Détection d'empaqueteur nommée

Fait : `packer.rs` — UPX, PyInstaller, AppImage, par nom de section ou
nombre magique, **avec la preuve et son adresse**, et ce qu'on en fait
(`upx -d`, `pyinstxtractor`). Nommé dans le panneau Entropie, qui est
celui qui posait la question, et relevé dans les signaux.

Le nombre magique d'AppImage n'est cherché qu'à sa place, au décalage 8 :
trois octets se rencontrent par hasard dans n'importe quelles données. Et
`shc` a été retiré de la table avant d'y entrer, faute d'une marque dont
on soit sûr.

### A5 — [FAIT le 1er octobre] Le désassemblage, le flot, et le désalignement

Vérifié d'abord, et mesuré : le désassemblage est **purement linéaire**,
et c'est le bon choix pour un listing. Sur la figure classique, il rend
`call 0FFFF…h` puis `(bad)` là où le processeur lit `xor eax,eax ; inc
rax ; ret` — et le graphe de flot **perdait l'arête** du saut sans rien
dire.

Fait : `misalign.rs` relève les endroits où un saut atterrit au milieu
d'une instruction du listing, et le panneau « Figures du code » donne les
**deux lectures côte à côte**. Le listing n'est pas corrigé : un listing
troué serait pire que trompeur.

Deux bornes, chacune venue d'une mesure sur `/bin/ls` : les sections
exécutables seulement, et les sauts de moins de seize octets. Sans elles,
cinq figures relevées et aucune vraie — des chaînes de format décodées
comme du code.

Et une erreur que seule la capture d'écran a trouvée : le relevé était
juste dans les tests et ne montrait rien dans l'application, parce que
les zones de code étaient calculées en décalage de fichier alors que
`load_elf` pose chaque section à son **adresse virtuelle**.

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

- **[FAIT le 1er octobre] Naviguer depuis les panneaux vers le listing** :
  `ReRequest::GoTo`, et les adresses cliquables dans cinq panneaux. Au
  passage, un défaut que personne ne pouvait voir : le compte d'appels des
  imports valait **zéro** sur tout binaire ouvert, parce qu'il convertissait
  une adresse virtuelle en position de fichier alors que le listing d'un ELF
  ouvert est déjà en adresses virtuelles. 719 appels au lieu de 0 sur
  `/bin/ls`.
- **[FAIT le 1er octobre] Exporter un rapport d'analyse** : les boutons
  « Copier le rapport » et « Enregistrer… » existaient ; le rapport porte
  maintenant les signaux, le portrait par familles, la famille de chaque
  import, les indicateurs par nature et les deux lectures d'un listing
  trompé.
- **Les raccourcis repris par le sens** (A 7.1) : le jeu Vim force la main
  (`n` court-circuite `Espace n i`). Refonte proposée de longue date.

---

## Priorisation suggérée

1. ~~**C2**~~ — fait le 1er octobre, et quatre paniques de plus trouvées.
2. ~~**A1 + A2**~~ — fait le 1er octobre.
3. ~~**A3**~~ — fait le 1er octobre. ~~**A4**~~ et ~~**C3**~~ aussi.
4. ~~**A5**~~ — fait le 1er octobre. **Toute la première moitié de ce
   relevé est faite.**
5. **B1** — la virtualisation, quand le confort sur gros binaires le
   justifie. Gros, à décider.
6. **B2** — la modification de binaire, par petits pas réversibles.

Ce que la séance du 1er octobre a appris, et qui n'était pas dans ce
relevé : **la capture d'écran voit ce que la mesure ne voit pas.** Le
faux positif sur `raise` ne violait aucun invariant — il était juste
faux, et il a fallu regarder l'écran sur un vrai binaire pour s'en
apercevoir.

Rien de ce qui précède ne casse l'existant : les corrections sont faites
et testées, les améliorations se branchent sur des données déjà produites,
et le gros œuvre est chiffré avant d'être entrepris.
