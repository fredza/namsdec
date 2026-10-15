# namsdec — ce qui est fait, ce qui reste

Cette liste est l'**index** du travail. Le *pourquoi* de chaque entrée —
ce qui l'a fait apparaître, ce qu'elle coûte, ce qui a été essayé et
retiré — reste dans `AMELIORATIONS.txt`, qui ne bouge pas de rôle : une
seule source pour le raisonnement, une seule pour l'état. Les renvois
`(A 4.14)` pointent vers l'entrée correspondante de ce fichier-là.

Tenue à jour au fil des séances. Dernière mise à jour : **14 octobre
2026**, version 0.2.9-beta.1, 1 213 tests au vert.

- `[x]` fait et vérifié — un contrôle le retient
- `[~]` commencé, le reste est écrit sous l'entrée
- `[ ]` à faire

---

## Où en est l'interface

**Non, l'UX/UI n'est pas terminée.** Une passe complète a été faite le
25 septembre : seize scènes capturées sur profil neuf, deux relevés de
mesure sur les trente-trois scènes du banc, **vingt et un écarts corrigés,
dix-sept contrôles nouveaux et un élargi**. Ce que cette passe a mesuré
et **pas** corrigé est en bas de la section suivante, chiffré. C'est du
travail identifié, pas de l'exploration : il n'y a plus de relevé à
refaire.

Ce qui reste est de la finition mesurée : des alignements de quelques
pixels, deux tailles de police, et le vide sous « Prochaine
instruction », qui tient au même nœud que le vide de 56 px — l'état du
processeur est ancré.

---

## 15. AVX-512 : le morceau qui se découpait (A 7.21)

Le chantier paraissait d'un bloc. La mesure a montré que la grande
majorité des occurrences sont sur **ymm et xmm** : des alias EVEX
d'instructions déjà exécutées, écrites par un compilateur qui se sert
de l'encodage pour ses trente-deux registres, pas pour ses masques.

- [x] **`vmovdqu8/16/32/64`, `vmovdqa32/64`, `vpxord/q`, `vpandd/q`,
      `vpandnd/q`, `vpord/q`** rejoignent les bras qui les exécutaient
      déjà — plus de 1 200 occurrences pour une liste de noms.
- [x] **Une instruction masquée S'ARRÊTE et le dit** — avant même de
      regarder le mnémonique. L'ignorer donnerait un résultat faux sans
      le dire, et c'est pire que de refuser.
- [x] **Les registres `k0`–`k7`** (A 7.22) — huit entiers de 64 bits,
      `kmov*`, les combinaisons, `kortest*`/`ktest*`, et ce qui les
      remplit : `vptestm*`, `vptestnm*`, `vpcmp*` avec leur prédicat.
      Montrés en binaire dans la fenêtre des registres étendus, et
      seulement quand il y en a.
- [x] **Les registres `zmm`** (A 7.23) — trente-deux registres de 512
      bits, la règle du haut étendue (VEX efface, SSE préserve), et le
      coût de l'historique mesuré : 2 352 octets par jeu, 11 Mo pour
      cinq mille pas, borné par un contrôle.
- [x] **Le masquage des opérations** (A 7.24) — fusion et mise à zéro,
      posées dans `write_vec_op` : toutes les instructions y passent, et
      le masquage est une opération d'écriture. Ce n'était donc pas un
      travail sur chacune. Une famille dont on ne sait pas nommer
      l'élément s'arrête encore, plutôt que de décaler les voies.

---

## 14. Les instructions de chaîne de SSE4.2 (A 7.20)

Ce qui restait chiffré de 7.15, et le chemin suivant après `pmaxub` :
`pcmpistri` ouvre les boucles de `strlen`, `strchr`, `strcmp` et
`strstr` dans la bibliothèque C.

- [x] **Les quatre formes** — `pcmpistri`, `pcmpistrm`, `pcmpestri`,
      `pcmpestrm`, et leurs `v*`.
- [x] **Les quatre questions**, avec leurs règles de validité, qui ne
      sont pas les mêmes selon la question.
- [x] **Les quatre indicateurs** (CF, ZF, SF, OF) : c'est sur eux que
      la bibliothèque C branche.
- [x] **Plus une seule instruction manquante** dans les neuf binaires
      d'épreuve, hors AVX-512.

---

## 13. La sortie lue comme un terminal (A 7.19)

Le défaut signalé en bas de 7.17 et laissé ouvert : `□[2J□[H□[36m` se
lisait en tête du logo, dans la console comme dans la boîte.

- [x] **`ansi.rs`** — couleurs (16 + palettes étendues ramenées aux
      huit), effacement, fin de ligne, colonne du curseur, retour
      chariot, tabulation, retour arrière. Ce qui n'est pas reconnu
      disparaît au lieu de s'afficher.
- [x] **Peint en un seul bloc**, par un `LayoutJob` : une étiquette par
      morceau coûterait des milliers de widgets, et la sélection ne
      traverserait pas de l'une à l'autre.
- [x] **La boîte de saisie coupe sur les lignes de l'écran**, et non
      plus sur le texte brut — deux découpes seraient deux vérités.
- [x] **Dix contrôles**, dont un de bout en bout qui exécute un
      programme en couleur et mesure la couleur peinte dans la console.
- [ ] **Pas un terminal complet** : ni hauteur d'écran fixe, ni curseur
      qui remonte dans ce qui est déjà écrit. `ESC[A` ne fait donc rien.

---

## 12. La calculatrice compte en bits (A 7.18)

Demandé avec la calculatrice de GNOME en mode Programmation posée à côté
de la nôtre : le même style, plus de fonctions de bits.

- [x] **Vingt-neuf fonctions de plus**, quarante-sept en tout (A 7.18) —
      lire et changer un bit, extraire un champ (`bits(x, h, b)`),
      renverser l'ordre, les compléments, `parity` (celle du x86),
      `ffs`/`log2`/`ispow2`/`nextpow2`, les rotations sur 8/16/32 bits,
      `sar`, `smin`/`smax`, `mulhi`.
- [x] **Six familles, une à la fois** — onglets courts, phrase entière
      au survol. À plat, les quarante-sept poussaient le bas de la
      fenêtre hors de l'écran : mesuré à 1103 px pour un écran de 1100.
- [x] **La famille ouverte est un réglage** — elle revient au lancement
      suivant ; une famille inconnue retombe sur la première.
- [x] **Trois contrôles**, dont un qui refuse un argument de trop et un
      qui vérifie que ce qui n'est pas ouvert n'est pas peint.
- [x] **Le guide compte les fonctions dans le code** — il disait
      « dix-huit » longtemps après qu'il y en avait quarante-sept.

---

## 11. Une fenêtre complète, et ce qui se clique (A 7.17)

Deux griefs sur capture, le 15 octobre : la boîte de saisie montrait
quatre lignes d'un logo qui en fait sept, et rien ne disait quels textes
se cliquent.

- [x] **La boîte montre tout ce que le programme a écrit** (A 7.17) —
      largeur de la plus longue ligne, hauteur de toutes, bornées à
      l'écran ; au-delà c'est la fin qu'on voit, et une ligne dit ce qui
      reste au-dessus. Le calcul est dans `fin_qui_tient`, à part pour
      être éprouvé : une ligne trop large se replie et compte double.
- [x] **Deux signes pour tout ce qui se clique** — la main, et le trait
      au survol. Posés par `cliquable` (interface) et `clickable_label`
      (panneaux) ; une rangée entière se souligne entière.
- [x] **Un contrôle qui lit les sources des DEUX crates** — aucune
      étiquette cliquable hors des helpers. Il a trouvé deux oublis du
      premier coup.
- [x] **Un contrôle de mesure** — le trait est compté sur ce qui est
      peint, souris posée puis retirée ; éprouvé en le retirant.
- [x] **Les séquences ANSI sont interprétées** (A 7.19) — couleurs,
      effacement, retour chariot qui recouvre sa ligne, colonne du
      curseur ; le reste est jeté plutôt qu'affiché. Les huit couleurs
      d'un terminal sont des rôles : namsdec les prend dans son thème.

---

## 10. « Aller à cette adresse » : où, au juste ? (A 7.16)

Un grief de Fred sur capture : le clic sur une adresse menait toujours
au listing, alors qu'une chaîne, une table ou un bourrage se lisent dans
les octets. Et la fenêtre d'analyse devait retrouver son bouton
agrandir, en pastille à la macOS.

- [x] **Le clic demande où** (A 7.16) — deux entrées, « dans le code
      désassemblé » et « dans l'éditeur binaire ». La destination
      voyage avec la demande (`ReRequest::GoTo { addr, target }`) ;
      aller au code **ferme** la fenêtre d'analyse, aller aux octets la
      garde ouverte. Le choix ne se pose pas là où le contexte dit déjà
      de quoi il s'agit — boîte du graphe d'appels, adresse dans du
      pseudo-code.
- [x] **L'éditeur binaire sait où on l'envoie** — `ReResult::ShowBytesAt`
      cadre la page, choisit l'octet, et écrit l'adresse d'où l'on vient.
- [x] **Position ET adresse**, deux colonnes — la seconde seulement si
      elle apprend quelque chose (`adresses_utiles`), et « Aller à »
      accepte les deux.
- [x] **La pastille verte revient sur la fenêtre d'analyse** — et
      l'agrandissement s'arrête sous la barre de menus, qu'il
      recouvrait.
- [x] **Six contrôles**, dont deux au clic réel et un qui juge l'ordre
      des pastilles.

---

## 9. Le calcul vectoriel entier, au complet (A 7.15)

Un grief sur capture : FROGMAN s'arrête sur « l'interpréteur namsdec ne
gère pas l'instruction Pmaxub », juste après avoir demandé sa clé. Ce
n'est pas une curiosité : `pminub` est la pièce maîtresse du `strlen` de
la bibliothèque C, et tout binaire qui lit une chaîne passe par là.

- [x] **Soixante-dix mnémoniques de plus** (A 7.15) — min/max dans les
      deux signes et les trois largeurs, l'arithmétique saturée,
      `pshufb`, `palignr`, les `pack*`, les `pmovzx*`/`pmovsx*`, les
      produits et leurs deux moitiés, `pmaddwd`, les horizontales,
      `pavgb`, `psadbw`, `pabs*`, `pinsr*`/`pextr*`, `pcmpeqq`,
      `pcmpgtq`, `ptest`, les déplacements non temporels, et les
      `prefetch*` qui passent sans rien faire. Les formes AVX suivent :
      seul l'encodage change.
- [x] **Ce qui manquait a été MESURÉ, pas supposé** — les mnémoniques du
      moteur confrontés à ce qu'`objdump` trouve dans les neuf binaires
      d'épreuve, par fréquence : `palignr` 574, `pminub` 511, `movntps`
      160, `prefetch*` 444.
- [x] **Six contrôles de bout en bout**, assemblés par le vrai nasm :
      `the_vector_strlen_of_a_real_libc_runs`,
      `packed_min_and_max_know_their_sign_and_their_width` (bâti sur la
      dualité qui fait que `pmaxub` et `pminsb` rendent le même vecteur),
      `saturating_arithmetic_stops_at_the_edge`,
      `packing_shuffling_and_widening_move_the_bytes_where_they_belong`,
      `packed_multiplication_and_measures`,
      `one_lane_at_a_time_and_what_ptest_answers`,
      `multiply_add_and_horizontal_sums`.
- [x] **Le manuel ne peut plus être en retard sur l'interpréteur** —
      huit fiches disaient « namsdec ne les simule pas » de ce qui
      s'exécute maintenant. `the_manual_does_not_claim_an_instruction_is_unsupported_when_it_runs`
      assemble l'exemple de chaque fiche qui l'affirme et la dénonce si
      elle passe ; il compte ce qu'il a jugé et refuse de passer à vide.
      Éprouvé en semant une fiche menteuse.
- [x] **Sept fiches nouvelles**, le manuel passe de 164 à 171 — exigées
      par `everything_the_emulator_runs_has_a_sheet`, qui avait raison.
- [x] **La fenêtre d'aide ne garde plus sa taille quoi qu'elle montre**
      — trouvé par la checklist d'avant-commit : les scènes du banc
      ouvraient l'aide sur son sommaire et jamais sur une fiche. Une
      scène de plus (`pminub`, la fiche aux vingt-quatre mnémoniques), et
      `a_window_opens_at_the_size_of_what_it_shows` a nommé 103 px de
      vide au-dessus du bouton « OK ».
- [x] **`pcmpistri` et ses trois sœurs** (A 7.20) — les quatre
      questions de SSE4.2, leurs règles de validité, leurs quatre
      indicateurs, et le SEIZE qui fait sortir des boucles. Trois
      contrôles de bout en bout, écrits sur les usages réels : strlen,
      strchr, strstr, strcmp, une classe de caractères.
- [x] **AVX-512** : complet pour ce que namsdec rencontre (A 7.21 à 7.24) — les alias
      EVEX sur xmm/ymm des instructions déjà exécutées, soit plus de
      mille deux cents occurrences, et un refus net de toute
      instruction masquée. Restent les registres `k` (1 100
      occurrences) et les `zmm` (350).

---

## 8. Le listing ne se met plus en page d'un bloc (A 7.14)

Le gros chantier de performance laissé ouvert depuis le 1er octobre :
ne poser que les lignes visibles. Fait le 14 octobre 2026, en mode
NORMAL seulement — le champ de texte y est déjà non interactif, et rien
de ce qu'il porte (saisie, sélection, presse-papier) n'y existe.

- [x] **La vue ligne à ligne** (A 5.3, 7.8, 7.14) — `vue_listing.rs`.
      Sur un listing d'un million de lignes : première image 2 942 ms →
      **116 ms**, régime 11,3 ms → **1,86 ms**, et la mémoire
      **2 032 Mo → 268 Mo** (le galley d'un tel listing pesait 1,8 Go).
      Sur crackme-01 : 343 ms → 13,5 ms, et la recherche ouverte
      3 735 ms → 214 µs.
- [x] **Les deux vues posent le texte au même endroit** — retenu par
      `both_views_lay_the_text_in_the_same_place`, à un demi-pixel près,
      éprouvé en remettant le défaut. Le pas est PRIS sur une ligne mise
      en page (la hauteur de police dit 15,125 là où les rangées se
      suivent de 15,0) et les colonnes LUES dans le galley de leur ligne
      (egui cale chaque glyphe sur le pixel).
- [x] **Une seule gouttière, un seul surlignage** — `Géométrie` répond
      aux mêmes questions qu'elle lise le galley du champ ou qu'elle
      calcule. Deux peintures qui dévieraient se verraient au changement
      de mode.
- [x] **Le texte entier n'est relevé qu'une fois par texte** — où
      commence chaque ligne, quelle est la plus longue, ce que le fichier
      définit. Retenu par `the_whole_text_is_surveyed_once_per_text`.
- [x] **La révision du texte, et qui a le droit de l'écrire** — les poses
      du listing servaient de preuve qu'il avait changé ; la vue leur
      retire ce rôle. `tout_ce_qui_ecrit_le_texte_le_dit` lit les
      vingt-neuf fichiers de l'interface et a trouvé du premier coup un
      site oublié.
- [x] **Cliquer dans le code y pose le curseur** — impossible jusqu'ici
      en mode Normal, le champ y étant non interactif.
- [x] **La gouttière suit le fichier** — trouvé sur capture : la pastille
      d'un point d'arrêt recouvrait le numéro (« ●012 » pour la ligne
      4 012), et sept chiffres n'y tenaient pas du tout.
- [ ] **Entrer en Insertion sur un million de lignes** paie encore les
      trois secondes et les 1,8 Go : c'est le vrai champ qui revient. Le
      cas est rare — on n'édite pas un désassemblage — et il est écrit.

## 7. Les gros binaires ne gèlent plus — passe du 14 octobre (A 7.13)

Quatre gels mesurés, quatre fois la même cause : un travail
proportionnel au listing entier, refait à chaque image, pour en montrer
trente lignes. Aucun ne se voit sur crackme-01 ; tous se voient sur un
binaire de quatre mégaoctets.

- [x] **La recherche ne gèle plus l'application** (A 7.13) — 3 735 ms
      par image barre ouverte sur crackme-01 (24 081 occurrences de
      « mov »), contre 0,8 ms fermée ; sur croackpocalypse, l'image ne
      venait plus. Les occurrences sont relevées une fois, portent leur
      ligne et leur colonne (`text::Place`), et seule la tranche visible
      se peint. Retenu par
      `the_search_hits_are_gathered_once_and_kept` : six images, un
      relevé.
- [x] **La fenêtre Désassemblage ne redécode plus tout** (A 7.13) — elle
      composait une ligne par instruction du programme à chaque image, un
      décodage et deux chaînes chacune, alors que sa zone en montre
      quarante. Retenu par
      `the_disassembly_window_decodes_once_per_question`.
- [x] **La gouttière trouve ses rangées par dichotomie** (A 7.13) — 8,96
      ms par image pour retenir 61 rangées sur 1 098 627. Retenu par
      `the_visible_rows_are_the_same_the_dichotomy_finds_them`, qui
      vérifie que la dichotomie désigne EXACTEMENT ce que le parcours
      désignait.
- [x] **Le texte n'est reconnu qu'une fois par image** (A 7.13) — le
      cache de mise en page compare le texte entier pour le reconnaître,
      1,5 ms sur seize mégaoctets, et c'était demandé deux fois par
      image. En mode Normal le champ n'est pas interactif : la mise en
      page lui est prêtée. Retenu par
      `the_listing_is_recognised_once_per_frame`.
- [x] **« Y a-t-il quelque chose à perdre ? » une fois par image** (A
      7.13) — la question comparait deux textes entiers, jusqu'à six fois
      par image. Relevée une fois pour tout ce qui AFFICHE ; ce qui
      DÉCIDE compare toujours sur le moment, et c'est
      `assembling_is_greyed_out_on_a_binary_listing` qui l'a imposé.
- [x] **Le chargement ne cherche plus les étiquettes en quadratique**
      (A 7.13) — `annotated_listing` relisait toute la table des symboles
      par instruction ; les deux listes étant triées, on avance dans les
      deux ensemble.
- [x] **Les compteurs de vim** (A 1.2, 7.13) — `5j`, `3w`, `12k`. Le
      compteur en attente se voit à côté de la pastille de mode
      (`a_pending_count_is_shown`), il est borné à quatre chiffres, et
      `move_cursor_times` ne découpe le texte qu'une fois pour toute la
      répétition — il le recopiait entièrement à chaque frappe de `j`.
- [x] **La ligne qu'on lisait revient** (A 4.11, 7.13) — sous la même
      garde que les points d'arrêt, curseur posé avant que l'éditeur se
      dessine, et sans changer de mode : on rouvre un binaire pour le
      lire. Retenu par `the_line_we_were_reading_comes_back`.
- [~] **La première image d'un gros listing** (A 5.3, 7.8, 7.13) — 2,9 s
      sur un million de lignes, et c'est toujours le même chantier :
      sortir le listing du `TextEdit`. Deux choses de plus à y mettre,
      mesurées : epaint parcourt lui aussi toutes les rangées du galley à
      chaque tesselation, et un galley d'un million de rangées pèse ce
      que pèsent ses glyphes.

## 6. Huit griefs sur captures — passe du 13 octobre (A 7.12)

Une liste de Fred, deux captures. Deux défauts se cachaient derrière les
autres, et c'est eux qui expliquaient la moitié de la liste.

- [x] **Le listing défile enfin jusqu'à l'adresse cliquée.** Le grief
      disait « dans figures de code et références croisées on ne sait
      pas aller aux adresses, pourtant l'infobulle dit aller à cette
      adresse ». Mesuré : le clic posait bien la demande, l'application
      la relevait, le curseur allait sur la bonne ligne — et la vue ne
      bougeait pas. egui ne défile vers le curseur d'un champ de texte
      que lorsque la sélection change PENDANT que le champ a le focus ;
      quand c'est l'application qui l'écrit, il n'a rien changé à
      l'image suivante. L'éditeur amène donc lui-même la ligne au
      centre. Contrôle : `going_to_a_line_really_scrolls_the_listing_to_it`.
- [x] **La fenêtre d'analyse s'efface au clic d'une adresse**, et la
      barre d'état dit où l'on arrive (`Status::WentTo`). Elle couvrait
      ce qu'on venait de demander à voir. Contrôle au CLIC, par
      kittest : `clicking_an_address_in_a_panel_really_lands_in_the_listing`
      — et il a attrapé la suite du défaut, l'état de la fenêtre étant
      écrit APRÈS le relevé des demandes, ce qui la rouvrait aussitôt.
- [x] **Plus de bouton agrandir/restaurer sur la fenêtre d'analyse**
      (demandé) : agrandie, elle recouvrait l'écran, et le listing
      défilait derrière un panneau opaque.
- [x] **La barre de chaleur « incomplète »** n'était pas incomplète :
      dans un binaire de soixante mille lignes, les lignes chaudes sont
      à soixante mille lignes du haut, et ni l'onglet Profil ni un
      panneau d'analyse n'y menaient. Réglé par le défilement ci-dessus.
- [x] **Les fonctions se renomment dans leur liste** — on clique le nom,
      on tape, Entrée ; la flèche de retour rend le `sub_xxxx`.
      L'adresse y mène au listing, comme ailleurs. Au repos le nom est
      une étiquette tronquée (quarante champs de saisie pèsent, et un
      nom long y serait coupé sans qu'on puisse le lire).
- [x] **Le renommage se répercute dans le listing** — il était écrit une
      fois à l'ouverture, à partir des symboles du fichier : le nom
      paraissait partout sauf là où l'on passe ses journées. Le listing
      se récrit, les points d'arrêt suivent leur adresse (une étiquette
      nouvelle décale tout ce qui suit), les symboles d'origine restent
      intacts, et un listing édité à la main n'est pas touché. Contrôle :
      `renaming_a_function_shows_up_in_the_listing`.
- [x] **Un panneau par outil extérieur** — Ghidra, RetDec, radare2,
      objdump, ndisasm —, chacun gardant sa sortie : c'est ce qui permet
      de comparer deux décompilateurs sur la même fonction. Générique
      (`ToolModule`), nourri par `known::all()` ; `ReRequest::RunTool` et
      `ReResult::{ToolOutput, ToolState}` ferment le canal. Rien ne
      s'exécute tout seul, la commande est écrite avant d'être lancée,
      un outil absent dit comment l'obtenir. Contrôle de bout en bout, au
      clic : `a_tool_panel_really_runs_its_tool_and_shows_the_output`.
- [x] **Un outil qui écrit beaucoup ne se bloque plus** — trouvé en
      branchant le précédent : `objdump -d /bin/ls` rend plusieurs
      méga-octets, le tube se remplit à soixante-quatre kilo-octets, et
      l'outil bloque en écriture pendant que namsdec attend sa fin. Deux
      fils lisent à mesure. Contrôles : `an_output_bigger_than_a_pipe_still_comes_back`,
      `a_tool_that_never_returns_is_stopped`.
- [x] **Fragments réutilisables (ROP) : une navigation** — deux mille
      fragments récoltés, quarante par page, « 41–80 sur 1 274 », et la
      recherche ramène en première page. La liste s'arrêtait à quarante
      sans dire qu'il y avait une suite.
- [x] **Entropie : compartimentée, graduée, et ses adresses** — la
      tranche suit la taille du binaire (256 o à 64 Ko, quelques
      centaines de tranches toujours) ; le profil prend toute la largeur
      et jamais plus de la moitié de la fenêtre ; une règle d'adresses
      court dessous ; l'échelle de couleurs est graduée de 0 à 8 avec
      neuf arrêts (cinq ne distinguaient pas un demi-bit dans la zone du
      code) ; et les **plages** donnent les adresses, cliquables, les
      miettes rejoignant leur voisine. Contrôles :
      `the_profile_never_takes_more_than_half_the_window`,
      `the_scale_tells_apart_what_half_a_bit_separates`,
      `the_panel_writes_its_ranges_and_where_they_are`,
      `the_window_grows_with_the_file_and_stays_round`,
      `the_ranges_join_what_says_the_same_thing`.
- [x] **Des carrés vides à la place de caractères** — trouvé en
      regardant la capture d'une nouvelle pagination : les polices
      qu'egui embarque n'ont ni `✓` ni `✗` ni les triangles ni les
      flèches. Six endroits les employaient, dont la liste des
      protections. Remplacés, et deux contrôles regardent désormais ce
      qui est PEINT, sur toutes les scènes :
      `tout_caractere_peint_existe_dans_la_police` (interface) et
      `tout_caractere_ecrit_existe_dans_la_police` (panneaux).
- [x] **La colonne de navigation tient ses vingt-quatre rubriques** —
      les noms y sont courts (« RetDec » et non « Décompilation par
      RetDec », qui passait sur deux lignes), les entrées serrées, et la
      fenêtre s'ouvre cent pixels plus haut. Nouveau `ReModule::nav_title`.
- [x] **Les fonctions empruntées sont des liens** (demandé) — survol :
      la phrase ; clic : la fiche, avec la signature et le registre qui
      porte chaque argument (`libc::ARGUMENT_REGISTERS`, désormais la
      seule source des noms de registres d'argument dans la crate —
      `explain.rs` en tirait une seconde liste). Quatre entrées
      nouvelles dans la table (`__libc_start_main`, `__cxa_finalize`,
      `__cxa_atexit`, `_Unwind_Resume`) : c'étaient les deux seules
      lignes du crackme de Fred, et elles étaient muettes. Et
      `libc::family_hint` dit ce que le PRÉFIXE d'un nom apprend sans
      rien deviner sur la fonction — `explain` garde son contrat.
      Contrôles : `le_nom_d_une_fonction_se_deplie_sur_sa_fiche` (qui a
      appris en chemin qu'egui ne peint pas ce qui sort de l'écran :
      la fiche se déplie sur la PREMIÈRE entrée affichée) et
      `une_fonction_inconnue_dit_ce_que_son_nom_apprend`.
- [x] **La fenêtre d'analyse ne grandit plus avec son contenu**
      (capture de Fred, rubrique « Chaînes lisibles ») — les chemins de
      source Rust, cent caractères avec leur empreinte de version,
      poussaient la fenêtre à 1 831 px de large contre 1 100 au départ,
      colonne de navigation sortie par la gauche. Et elle y restait :
      egui garde la taille d'une fenêtre. Deux bornes : le contenu
      défile désormais dans les DEUX sens (avec la largeur de lecture
      imposée avant la zone, sans quoi les panneaux qui se
      dimensionnent sur `available_width()` s'étireraient), et la
      fenêtre est bornée à ce qu'elle peut montrer — colonne plus
      largeur de lecture. Contrôle sur TOUTES les rubriques :
      `la_fenetre_d_analyse_ne_grandit_pas_avec_son_contenu`, éprouvé
      en remettant le défaut.
- [x] **Le processeur est un onglet** (demandé, option 1 sur trois
      proposées) — `SidebarTab::Cpu` en tête, vue par défaut, titre
      « État du processeur » dans le panneau et « Processeur » dans la
      rangée. Le panneau passe de trois zones à deux, et tout le
      calcul de partage lié à la hauteur des registres disparaît
      (`sidebar_cpu_height`, `cpu_state_open`, plancher de 280 px
      ramené à ce que le bandeau demande vraiment). La vue est bornée
      à la place du panneau — elle y défile plutôt que de déborder — et
      sa hauteur lui est imposée DANS la zone défilante, sans quoi les
      vues qui dimensionnent leur liste sur `available_height()`
      cessent de défiler elles-mêmes. Contrôle :
      `l_etat_du_processeur_est_un_onglet_et_la_vue_choisie_a_tout_le_panneau`.
- [x] **Les registres sur une colonne de plus quand le panneau est
      court** (demandé) — le blocage n'était pas la largeur des valeurs
      mais le BLANC entre colonnes (quatre espacements). Nouvelle
      fonction pure `register_layout` : elle rend le nombre de colonnes
      et le blanc, serre seulement si les registres ne tiennent pas en
      hauteur, et seulement si serrer gagne vraiment une colonne. Pas
      de boucle de mesure — la question posée ne dépend pas du nombre
      de colonnes choisi. Contrôles : les trois cas de
      `les_registres_gagnent_une_colonne_quand_le_panneau_est_court`,
      et `a_l_ecran_les_registres_se_rangent_sur_une_colonne_de_plus`
      qui compte les abscisses peintes. Limite mesurée et assumée : sur
      une fenêtre minimale (800×520), la largeur n'autorise que deux
      colonnes quoi qu'on fasse.
- [x] **L'onglet Octets suivait l'exécution de travers** — trouvé en
      corrigeant le reste : `show_rows` veut la hauteur d'une ligne
      SANS l'espace qui la sépare de la suivante, il l'ajoute lui-même.
      On lui donnait les deux : il croyait à des rangées de 21 px là où
      il en posait 27, et le décalage calculé pour centrer
      l'instruction courante tombait soixante-dix lignes trop haut sur
      un programme de quatre cents instructions. La fenêtre de
      décompilation portait déjà la remarque, pour la même raison.
- [x] **Le panneau latéral ne se recouvre plus, même à l'étroit**
      (capture de Fred, onglet Mémoire) — l'état du processeur
      s'écrivait sur le haut de la vue. Cause mesurée, et elle est dans
      egui : quand le contenu d'un panneau dépasse la place laissée,
      egui ramène la TAILLE du rectangle au maximum demandé en gardant
      le bord du CONTENU et non celui de la place — le rectangle glisse
      vers le bas, et le panneau ne rend au parent que ce qui reste
      visible (relevé : peint de 361 à 539, rendu de 403 à 539). Le
      contenu de la vue est borné à son panneau par une zone défilante,
      et le bandeau perd son plancher de 64 px (`min_scrolled_height`),
      qui le faisait déborder de vingt et un pixels sur un panneau
      court. Contrôle sur les cinq vues, panneau à l'étroit et poignée
      tirée : `le_panneau_lateral_ne_se_recouvre_jamais_meme_a_l_etroit`
      — éprouvé en remettant le défaut.
- [x] **Et elles sont rangées par familles** (demandé) — Le fichier, Le
      code, Décompiler, Ce qu'il dit, Ce qu'il emprunte, Les octets, dans
      l'ordre où l'on lit un binaire inconnu. Le module déclare la
      sienne (`ReModule::section`, `ReSection`), l'ordre
      d'enregistrement les tient groupées, et la colonne pose un
      intertitre quand la famille change. Deux rubriques ont déménagé
      pour rejoindre la leur : « Figures du code » et « Références
      croisées » étaient au milieu de ce que le binaire emprunte.
      Contrôle : `les_rubriques_d_une_meme_famille_se_suivent` — éprouvé
      en remettant le défaut, il nomme la rubrique fautive et sa
      famille.

## 5. Analyse et exercices — passe du 7 octobre (0.2.8-beta.1)

Cinq demandes de Fred, sur captures.

- [x] **Entropie en dégradé continu** — l'histogramme passait par quatre
      paliers de verdict ; il suit un dégradé vert → jaune → orange →
      rouge → violet sur l'échelle 0..8, couleurs fixes hors thème.
      Contrôle : `the_gradient_climbs_from_green_to_purple`.
- [x] **Flags plus espacés** dans la barre de titre (4 → 8 px) : collés,
      ils se lisaient comme un bloc.
- [x] **Un nom donné suit l'adresse partout** — `clickable_address`
      montre le nom à la main quand il y en a un (propagation à tous les
      panneaux), avec un léger fond en évidence, repris dans l'en-tête de
      l'arbre d'appels et la liste des fonctions. Pivot :
      `ReContext::user_name`.
- [x] **Le but et la progression d'un exercice** quittent le bandeau du
      processeur (toujours visible, d'où l'exercice resté affiché après
      chargement d'un binaire) pour la **fenêtre dédiée** (Aide ›
      Exercices). `goal_ui` passe `pub(crate)`.
- [x] **Rubrique YARA dans l'onglet RE** — nouveau `YaraModule` : tire une
      règle YARA du binaire (chaînes distinctives sans le bruit de
      bibliothèque, nombre magique, empreinte en méta, seuil prudent), à
      coller dans un `.yar`. Contrôles : `the_rule_is_well_formed` & co.

### Éditeur binaire (7 octobre)

- [x] **Début de l'éditeur binaire dans l'onglet RE** — nouveau module
      `HexEditorModule` (« Éditeur binaire »). Le fichier en grille
      hexadécimale (décalage · 16 octets · colonne de texte), navigable
      par page et par « Aller à ». On clique un octet, on en pose un
      autre : `75` (jne) → `74` (je) pour lire l'autre côté du saut. Ce
      qu'on a touché porte un fond. Chaque écriture passe par le journal
      des correctifs (réversible, Outils › Correctifs), et rien n'atteint
      le disque sans enregistrement explicite, vers un fichier choisi —
      jamais par-dessus l'original. Première version : l'édition multiple
      et l'insertion (qui décale les adresses) viendront ensuite.
      Contrôles : les trois de `hex_editor_module` +
      `l_editeur_binaire_dispose_le_fichier_en_grille`.
- [x] **Éditeur binaire — l'inspecteur de données (façon 010 Editor)** —
      sous l'octet choisi, les octets lus dans chaque format : int8…int64
      signés/non signés, flottants, caractère, bits, en petit-boutien.
      Chaque lecture se copie d'un clic. Contrôle :
      `l_inspecteur_lit_chaque_format`. Reste vers 010 Editor : frappe en
      place dans la grille, édition de la colonne de texte, curseur aux
flèches, bascule gros-boutien.
- [x] **Éditeur binaire — la recherche** — un motif en texte (« ELF ») ou
      en octets hexadécimaux (`7f 45 4c 46`), en avant ou en arrière, qui
      fait le tour, trouvaille surlignée. Fonctions pures `motif` et
      `trouver`. Contrôles : `un_motif_se_lit_en_hex_ou_en_texte`,
      `la_recherche_trouve_et_fait_le_tour`. (Étudié d'après 010 Editor
      16.0.4, dont on s'inspire des fonctions — jamais du code.)
- [x] **Éditeur binaire — clavier (façon 010 Editor)** — un octet choisi,
      les flèches déplacent le curseur (←→, ↑↓ d'une ligne, Page), et les
      chiffres hexa l'écrivent en place, quartet par quartet (7 puis 4 →
      74, avance tout seul). Échap annule un quartet. N'agit que si aucun
      champ de texte n'a le focus. Logique pure `frapper`, testée. Reste
      vers 010 Editor : édition de la colonne de texte, bascule
      gros-boutien.
- [x] **Éditeur binaire — la fenêtre n'enfle plus au clic d'un octet
      (8 octobre)** — l'inspecteur qui s'ouvre sous l'octet choisi était
      une `egui::Grid`. Dans la fenêtre d'analyse, qui s'ajuste à son
      contenu, toute rangée imbriquée (grille ou `ui.horizontal`) réserve
      la largeur disponible : la fenêtre grandit pour la contenir, ce qui
      agrandit la disponible, et l'on repart. Mesuré : 1100 px → 2878,
      puis deux de plus à chaque image, jusqu'à déborder de l'écran —
      centrée, sa moitié gauche passait hors champ et le contenu se
      coupait à gauche (capture de Fred). Bissecté jusqu'à la rangée.
      L'inspecteur est maintenant **un libellé par ligne**, une seule
      galère de texte (nom en chasse fixe atténué + valeur, via
      `LayoutJob`), qui ne lit aucune largeur. Contrôle :
      `choisir_un_octet_ne_fait_pas_enfler_la_fenetre`.
- [x] **Onglet Mémoire — « Lire en : » se peignait sur « Suivre : »
      (8 octobre)** — « Suivre : … » et « Lire en : … » tenaient sur une
      seule rangée `horizontal_wrapped`. Sur un panneau étroit elle
      débordait, et `horizontal_wrapped` donnait à l'élément reporté une
      galère démesurée (257 px pour les neuf caractères de « Lire en : »)
      peinte à l'abscisse de départ, par-dessus « Suivre : » et les
      boutons de suivi — cinq textes l'un sur l'autre, mesuré sur binaire
      chargé, barre latérale ~480 px, à tout facteur d'échelle. Deux
      rangées franches règlent l'affaire. Contrôle :
      `lire_en_ne_se_peint_pas_sur_suivre`.
- [x] **Onglet Mémoire — la carte dépliée se peignait sur les registres
      (8 octobre, le défaut de la capture de Fred)** — un binaire aux
      régions nombreuses (crackme-01 en a **34** ; les sections d'un
      programme lié dynamiquement) dépliait une carte si haute que le
      contenu de l'onglet dépassait la place que le panneau latéral lui
      réserve. Le partage à trois (bandeau, état du processeur, onglet)
      **divergeait** alors — même classe que le #8 : selon le chemin,
      l'onglet remontait par-dessus les registres (RAX…RIP recouverts par
      la carte, ce que Fred voyait) ou filait sous le bas de la fenêtre
      (mesuré : bas à 1646 px pour une fenêtre de 1183). La carte défile
      désormais dans sa propre fenêtre **bornée** (`max_height`) : trop
      haute, elle ne peut plus pousser le reste. Reproduit via les
      34 régions de crackme ; contrôle :
      `la_carte_depliee_ne_couvre_pas_les_registres`.
- [x] **Le nom du binaire s'affiche sur l'onglet (8 octobre)** — un
      binaire ouvert n'a pas de `path` (on ne le rouvre pas en
      l'enregistrant par-dessus l'original), et l'onglet restait « sans
      titre ». Son nom voyage pourtant dans `from_binary` : c'est lui qui
      s'écrit maintenant, sur l'onglet actif comme sur ceux rangés.
      Contrôle : `l_onglet_d_un_binaire_porte_son_nom`.
- [x] **Le chargement d'un gros binaire ne fige plus l'interface
      (8 octobre)** — ouvrir un binaire de 3 Mo bloquait le fil
      d'interface **quatre secondes** (bien plus en build debug), au point
      que le système proposait de « forcer à quitter ». Mesuré : tout le
      temps était dans `on_binary_loaded`, et dedans dans
      `functions::discover`, qui rebalayait les 400 000 instructions **pour
      chaque** fonction découverte — quadratique (le module « Fonctions »
      et le graphe d'appels l'appellent tous deux). Remplacé par une
      attribution dichotomique en un passage : `load_elf` passe de **4,0 s
      à 0,63 s** sur le même binaire. Même gain au Reset, qui refait la
      découverte. Garde-fou : `discovering_functions_stays_linear_on_a_big_binary`.
      Reste : la 1re image après chargement coûte ~1,4 s (rendu du listing
      de 400 000 lignes) — moins grave, autre sous-système.
- [x] **Commentaire périmé de `flags_ui` corrigé** — il disait encore
      « chacune se bascule d'un clic » depuis que les flags sont en
      lecture seule. (L'édition de registres, elle, est gardée — choix de
      Fred : c'est le `set \$reg` de gdb, réversible par Step Back.)
- [x] **Fausse alerte élucidée (8 octobre)** : je craignais que l'analyse
      reste périmée après un patch, parce que `load_elf` n'appelle pas
      `on_binary_loaded`. Vérifié empiriquement : elle se rafraîchit bien.
      `load_elf` appelle `rebuild_emulator`, qui dispatche vers
      `load_process` ou `rebuild_from_assembly` — et ces deux-là notifient
      les modules. Le SHA-256 change après une écriture. Verrouillé par
      `une_ecriture_rafraichit_l_analyse`, qui tombe si l'on casse la
      chaîne. Leçon : tracer jusqu'au bout avant d'inscrire un défaut.

- [x] **Vérifié (8 octobre) : les numéros de ligne ↔ les instructions** —
      demande de Fred. Alignés par construction : `layout_code` pose le
      texte à largeur de repli INFINIE, donc aucune ligne ne se replie,
      `galley.rows.len()` = nombre de lignes source, et le numéro `i+1`
      est la vraie ligne source ; la surbrillance d'exécution vient
      d'`addr_to_line[rip]`, la même ligne. Verrouillé par
      `les_numeros_de_la_gouttiere_suivent_les_lignes_source` (une ligne
      de 400 caractères en tête, numéros peints consécutifs à pas
      constant — un repli régressif les ferait sauter).

- [x] **Éditeur binaire — parité 010 Editor atteinte (8 octobre)** : deux
  dernières pièces. Édition dans la **colonne de texte** — Tab bascule de
  la colonne hexa à la colonne texte, où une frappe pose le caractère
  (via `Event::Text`), curseur partagé, même chemin d'écriture réversible.
  Et la **bascule gros-boutien** de l'inspecteur : le même quadruplet se
  relit dans l'autre sens, étiquettes LE/BE à l'appui. `inspecter(octets,
  be)` et l'enum `Colonne`. Contrôle `l_inspecteur_lit_chaque_format`
  étendu au gros-boutien.

### Pont DZF — le contrat JSON (8 octobre)

- [x] **Le contrat de décompilation DZF ↔ namsdec, figé.** DZF reçoit les
  octets et l'adresse, rend une décompilation en JSON ; namsdec la lit.
  Côté namsdec : `crates/namsdec-re/src/dzf.rs` — types serde tolérants
  (`DzfOutput`, `DzfFunction`) + `parse`, la source de vérité. Pour Astra :
  `docs/contrat-dzf.md`, la spec en français (appel du plugin, forme du
  JSON champ par champ, règles de tolérance). Principes : adresses
  virtuelles u64, `address` seule exigence dure, champs inconnus ignorés
  (DZF grandit sans casser namsdec), `version` pour les changements de
  sémantique, vide valide. Cinq contrôles (`un_exemple_complet_se_lit`,
  `le_minimum_suffit`, `un_champ_inconnu_est_ignore`,
  `une_fonction_sans_adresse_est_refusee`, `le_vide_se_lit`).
- [x] **Le panneau décompilateur DZF** (`DzfDecompilerModule`,
  « Décompilation DZF ») — rend le JSON du contrat : chaque fonction en
  pseudo-C dans un repli, sa signature, son adresse cliquable. Branché sur
  les **noms partagés** : une fonction rebaptisée dans l'arbre d'appels
  porte son nom namsdec ici aussi (prouvé :
  `le_nom_de_namsdec_l_emporte_sur_celui_de_dzf`). État vide clair tant que
  DZF n'a rien rendu. `set_decompilation` reçoit la sortie ;
  `take_request` relaie l'aller-à. Enregistré après le graphe d'appels.
- [x] **Câblage de bout en bout (8 octobre)** : le bouton « Décompiler »
  du panneau émet `ReRequest::Decompile` ; le GUI lance le décompilateur
  actif (`start_decompile`, en tâche de fond), et `poll_decompile` tente de
  lire sa sortie comme du JSON DZF (`ingest_dzf_json`) pour la livrer au
  panneau via le nouveau canal retour `ReResult` (`deliver`). Une sortie de
  texte (rizin/retdec) n'est pas livrée ; une demande DZF restée sans JSON
  se dit. Symétrie posée : `ReRequest` (module→app), `ReResult` (app→
  module). Éprouvé avec un « faux DZF » (JSON fourni à la main) :
  `le_json_de_dzf_livre_au_panneau_s_y_affiche`,
  `une_sortie_non_dzf_demandee_le_dit`.
- [x] **Adresses cliquables dans le pseudo-code (8 octobre)** —
  `0x401030`, `sub_401030`, `loc_401030` mènent au listing, à l'endroit
  exact visé. Tout le pseudo-code est peint en **une seule galère**
  (aucun `ui.horizontal` imbriqué, qui ferait gonfler la fenêtre
  d'analyse — cf. le #8) : les adresses sont colorées en lien, et la
  position du clic est testée contre les plages de caractères pour sauter
  à la bonne. Fonction pure `adresses_cliquables`, trois contrôles.
  Compromis : le pseudo-code n'est plus sélectionnable d'un bloc (un
  libellé cliquable n'est pas un libellé sélectionnable) — à revoir si le
  copier-coller manque.
- [ ] **À suivre** : éprouver avec le vrai DZF quand Astra l'aura fait
  émettre du conforme.

### Revue d'interface — l'éditeur binaire et les modules (8 octobre)

Audit egui (skill audit-ui-egui), concentré sur la surface neuve :
l'éditeur binaire (grille, inspecteur, recherche, clavier) et YARA.

- **Mesuré, propre.** Contraste du texte des octets sur les trois fonds de
  surlignage : 5,7–6,3 (le bleu de sélection #264F78 est assez sombre).
  Recouvrement : 0 sur 330 textes de l'éditeur. Découpage : 0. Les faux
  positifs écartés : le bloc « octet choisi » non capturé était SOUS le
  pli (joignable au défilement de la fenêtre d'analyse), pas découpé.
- [x] **Vrai manque trouvé et comblé : les scènes n'éprouvaient que le
  module d'analyse PAR DÉFAUT.** `fenêtres_ouvrables` ouvrait la fenêtre
  d'analyse sans choisir de module ; l'éditeur binaire, YARA, le graphe
  d'appels et la plupart des autres passaient au travers des SEPT
  invariants de scène (recouvrement, dérive, balisage, doublons, hauteur
  des boutons, panneau vide, taille d'ouverture). `scènes()` ajoute
  désormais une scène PAR MODULE (en marche) — tous éprouvés, un module
  neuf couvert sans qu'on y pense. Les sept passent sur les dix-sept
  modules.

### Passe de vérification (7 octobre)

Relecture à la recherche d'erreurs. Ce qui tenait : les analyseurs ne
paniquent sur aucun en-tête ELF incohérent (relevé jeté sur quinze points
d'entrée), le parcours du graphe d'appels reste fini, la détection d'un
outil installé balaie bien tous les sous-dossiers, les 791 chaînes des
deux langues sont traduites, et les deux séries d'exercices sont
exécutées par un contrôle. Ce qui ne tenait pas :

- [x] **Le manuel nommait six panneaux autrement que l'écran** —
      « Gadgets » pour *Fragments réutilisables (ROP)*, « Fonctions
      empruntées » pour *Fonctions importées*, « Chaînes » pour *Chaînes
      lisibles*, et trois noms tronqués (*Figures du code*, *Protections
      du binaire*, *Place libre (bourrage)*). On cherchait dans
      l'interface un nom qui n'y était pas. Neuf mentions corrigées, et
      un garde-fou qui lit les titres **dans le registre** :
      `le_manuel_nomme_les_panneaux_comme_l_interface`.
- [x] **Le journal des versions oubliait la moitié de 0.2.8** — l'entrée
      ne couvrait que les cinq dernières demandes et taisait tout ce qui
      était entré depuis 0.2.6 : le graphe d'appels, le renommage, les
      adresses cliquables, les signaux, la place libre, le désaccord de
      lecture, « En bref », le sommaire des 674 mnémoniques, DZF
      installable, la panique sur ELF hostile. Entrée complétée.
- [x] **L'en-tête de ce fichier annonçait 1 087 tests** pour 1 099.
- [x] **Quatre réglages que l'interface bascule n'étaient pas gardés** —
      l'explorateur de fichiers (alors que la console, la barre latérale
      et la barre d'état, ses frères, se souvenaient), le filtre du graphe
      de flot, celui du déroulement, et le repli des variables
      d'environnement. Le manuel promettait pourtant « tout réglage est
      enregistré » : c'est le code qui a été mis à la promesse. Garde-fou
      qui lit le code qui bascule et exige la correspondance dans les deux
      sens : `tout_reglage_bascule_par_l_interface_est_garde`.
- [x] **Les fichiers que namsdec réécrit n'étaient pas remplacés d'un
      bloc** — `fs::write` tronque puis écrit. Une coupure entre les deux
      laissait un `settings.json` vide, que la lecture suivante remplaçait
      par les défauts, en silence. Même piège pour les raccourcis, les
      **notes par binaire** (les noms donnés aux fonctions, parfois des
      heures de lecture) et le source de l'utilisateur. Écriture à côté,
      `sync`, puis renommage atomique : `write_atomically` dans
      `resources.rs`. Contrôle : `un_fichier_reecrit_est_remplace_d_un_bloc`.

---

## 1. Interface — corrigé

- [x] Les fenêtres reviennent ouvertes d'une séance à l'autre, en
      escalier (A 7.11)
- [x] Les trois zones du panneau de droite ont le même en-tête et le même bord
- [x] Un titre de zone se souligne, il ne s'encadre pas
- [x] Sept boutons sans liseré encadrés — et le contrôle qui les
      manquait, réparé
- [x] Deux boutons voisins ont la même hauteur
- [x] Les points de suspension sont des points de suspension (dix libellés)
- [x] Plus aucun écran ne montre son balisage (`**gras**` rendu)
- [x] Ce qui se dit « chasse fixe » l'est vraiment (quatre textes)
- [x] Préférences › Général garde un seul rythme
- [x] Une section dépliée dit toujours quelque chose
- [x] Deux mots identiques ne se touchent plus (« 100 % » → « Taille réelle »)
- [x] L'état du processeur ne défile jamais, à toute taille (A 7.11)
- [x] Le bandeau ne coupe plus de ligne en deux (A 7.11)
- [x] « Prochaine instruction » part repliée devant un binaire (A 7.11)
- [x] Chaque panneau vide dit ce qu'il montrera, dans ses propres mots
- [x] Les icônes ne se peignent plus en bouillie
- [x] Tout texte se détache de son fond (contraste mesuré, tous les thèmes)
- [x] Les boîtes de dialogue parlent la même langue (A 1.24)
- [x] Les fenêtres internes disent leur rang (A 2.1)
- [x] Six fenêtres s'agrandissent (A 2.9)
- [x] **Une commande grisée dit pourquoi elle l'est** — `on_hover_text`
      ne s'affiche pas sur un widget éteint, si bien que cinq boutons de
      la barre d'outils et six entrées de menu ne disaient rien au
      survol. C'est l'écran qu'on a sous les yeux tant qu'on n'a pas
      assemblé.
- [x] Un changement d'échelle de l'écran redemande une image
- [x] **On voit qu'on peut ouvrir plusieurs fichiers** — la barre
      d'onglets est là **dès le premier écran**, tampon « sans titre »
      compris, avec un « + » au bout. Elle n'apparaissait qu'à partir du
      second fichier : rien, jamais, ne disait que le mécanisme
      existait. Premier remède insuffisant — « dès le premier fichier
      OUVERT » — parce que l'écran d'accueil n'en a pas.
- [x] **La barre d'onglets ressemble à une barre d'onglets** — des
      rectangles francs collés à l'éditeur, liseré d'accent sur l'actif,
      croix à l'intérieur, rond du non-enregistré à sa place. C'étaient
      trois objets qui flottaient : une pastille arrondie, une croix
      détachée, un plus.
- [x] **Le « + » crée un programme neuf** — il menait au sélecteur de
      fichiers, puis à un menu de deux entrées ; les deux fois c'était
      trop. Et « Nouveau programme » ne créait PAS d'onglet quand le
      tampon courant était vierge : le geste n'avait aucun effet
      visible.
- [x] **Les tampons sans titre se numérotent** — trois programmes neufs
      s'appelaient tous « sans titre ».
- [x] **« Fermer le fichier » ne disparaît plus du menu** — elle
      s'effaçait quand aucun fichier n'était ouvert, c'est-à-dire au
      moment où on la cherche. Grisée, avec sa raison.
- [x] **namsdec répond à sa ligne de commande** — `namsdec fichier.asm`
      ouvre le fichier, `--version` et `--help` répondent sans ouvrir de
      fenêtre. Le lanceur déclarait `Exec=namsdec %f` et
      `MimeType=text/x-asm` : « Ouvrir avec namsdec » ouvrait une fenêtre
      vide, ce qui est pire que de ne rien proposer.
- [x] **Remettre une fenêtre devant marche aussi au premier plan** — les
      trois fenêtres qui y vivent (plan d'installation, « À propos »,
      saisie) recevaient un ordre adressé au rang ordinaire, donc à un
      calque vide portant leur nom.
- [x] « Touche simple… » et « Séquence… » sont des boutons encadrés — et
      le contrôle voit maintenant la troisième façon d'écrire un bouton,
      `egui::Button::new` posé à la main
- [x] Le plancher du bandeau suit la hauteur réelle de l'état du
      processeur (335 px) au lieu d'une constante de 280
- [x] La zone du bandeau prend la place qui lui reste et fait défiler ce
      qui n'y tient pas, au lieu d'épouser son contenu et de déborder
- [x] **Fichier > Fermer le fichier** marche aussi sur le dernier
      fichier ouvert, et l'entrée ne disparaît plus du menu

## 2. Interface — ce que la revue a mesuré et pas corrigé

Détail chiffré dans `AMELIORATIONS.txt` 7.11, section « REVUE DU
25 SEPTEMBRE — CE QUI RESTE ».

- [x] **Huit fenêtres plus grandes que ce qu'elles montrent** — une
      fenêtre s'ouvre maintenant à la taille de ce qu'elle a à montrer,
      et le contrôle mesure le plus grand vide entre deux bandes de
      contenu. Le graphe est passé de 863 px de trou à moins de 80.
- [x] **« Registres étendus » est trop grande même pleine** — 440 x 480
      pleine au lieu de 520 x 620, et 540 x 220 à vide.
- [x] **Les états vides ont deux formes** — une seule maintenant : le
      constat, puis l'explication, centrés l'un sur l'autre. Cinq
      fenêtres converties. Exception nommée : « Aucun décompilateur
      installé. » est un mode d'emploi, pas un état vide.
- [x] **Le même fait s'écrit de quatre façons** — deux maintenant :
      « Aucun binaire assemblé. » pour le bandeau et ses onglets,
      « Aucun programme lié. » pour les fenêtres qui flottent par-dessus.
      Deux, et non une : les trois pourraient être à l'écran en même
      temps, et le contrôle des répétitions veille.
- [x] **Le listing de désassemblage perd ses colonnes de 54,8 px** — la
      colonne des étiquettes a une largeur fixe, prise sur le plus long
      symbole et bornée à vingt signes.
- [x] **La barre de saisie de la console** : 4,5 px d'amplitude sur une
      seule rangée — l'étiquette venait en premier et se centrait sur sa
      propre hauteur. Le seul remède est que le premier élément soit
      déjà le plus haut : trente-quatre pixels, mesurés, et un contrôle
      les retient.
- [x] **Étiquette contre liste déroulante** : 3,5 px, partout où les
      deux se suivent. Six façons d'entourer la liste d'egui ont été
      essayées, l'écart n'a pas bougé d'un pixel — il est DANS
      `ComboBox`, dont le cadre part de `available_rect_before_wrap()`.
      Nous avons donc la nôtre : un cadre alloué et peint, le texte
      aligné à gauche, le chevron dessiné à droite. Elle se centre comme
      tout le reste, elle a le liseré des autres commandes, et un
      contrôle exige que l'étiquette et sa valeur se lisent sur la même
      ligne.
- [x] **Deux marges gauches dans le châssis**, 16 et 18 px — examiné,
      ce n'est pas un défaut : l'onglet « Console » est une pastille
      cliquable, son fond part à 16 comme le corps et son texte à 18
      parce qu'une pastille a un rembourrage. Le relevé comparait un
      texte nu à un texte encadré.
- [x] **La grille de bits de la calculatrice est en 12 pt** — elle prend
      maintenant le style Monospace du thème, comme les registres, la
      pile et le vidage. La case d'un bit fait neuf pixels quelle que
      soit la police : rien n'a bougé en largeur.
- [x] **Le graphe écrit ses instructions en 11 pt**, et « le programme
      est ici » y est en chasse fixe — examiné, gardé : dans une boîte
      où tout est en colonnes, l'annotation est cohérente avec son
      contexte, et la largeur des boîtes se calcule en colonnes de
      caractères. Changer la police casserait ce calcul.
- [x] **Le vide sous « Prochaine instruction »** — la cause n'était pas
      dans le partage, où trois tentatives l'avaient cherchée. Le panneau
      croyait que la poignée avait été tirée dès que la première image ne
      rendait pas la hauteur demandée, ce qui arrive toujours : cette
      image-là ne connaît encore ni la hauteur du bandeau ni celle de
      l'état du processeur. La part de l'onglet était gelée à son
      plancher pour toute la séance, et le bandeau gardait le reste. Une
      poignée ne bouge que sous un doigt.

## 2 bis. Le chevauchement des captures — élucidé

- [x] **Ce n'était pas un défaut de mise en page.** Les deux captures du
      25 septembre au soir montraient l'état du processeur et le haut de
      l'onglet Mémoire sur les mêmes pixels. Mesuré SUR L'IMAGE : « RIP »
      et « Carte de la mémoire » font 33 et 31 pixels de haut, les
      textes superposés à eux en font **dix-huit**. Deux échelles sur
      une même image — egui n'en dessine qu'une. C'est une bande restée
      d'un rendu précédent, plus petit, que rien n'a repeinte : le
      facteur d'échelle de l'écran avait changé entre les deux.
      Le banc ne pouvait pas le retrouver : il dessine à une échelle et
      une seule. Quatre hypothèses de mise en page ont été essayées
      avant celle-là ; deux ont trouvé de vrais défauts au passage (le
      plancher du bandeau, la zone qui débordait au lieu de défiler),
      deux ont été réfutées par la mesure.
      **Fait depuis** : un changement d'échelle redemande une image, ce
      qui évite d'attendre que le compositeur se réveille. On ne répare
      pas le compositeur, on ne l'attend pas non plus.

## 2 ter. Découvrabilité — première passe

- [x] **Toute action a un chemin qui se voit** — huit actions sur trente
      n'avaient que le clavier : dupliquer une ligne, surligner les
      occurrences, aller à la suivante et à la précédente, les renommer
      toutes, nommer une adresse, la commenter, revenir au fichier
      précédent. Elles sont dans les menus Édition et Outils, chacune
      avec son raccourci. Un contrôle le retient — c'est le pendant, du
      côté de l'œil, de « toute action a un raccourci dans les deux
      jeux ».
- [x] **Les quatorze fenêtres ont toutes un chemin** — vérifié : chacune
      s'ouvre depuis un menu ou un bouton, aucune n'est réservée à la
      palette.
- [x] **Ce qui se clique change le pointeur** — douze étiquettes
      cliquables, aucune ne le disait. Les infobulles annonçaient déjà ce
      qu'un clic ferait, mais il faut s'être arrêté dessus pour les lire,
      et l'on ne s'arrête que sur ce qu'on soupçonne d'être cliquable.
      egui réserve la main aux liens ; `cliquable` la pose, et un
      contrôle l'exige.
- [x] **Les poignées disent qu'elles se tirent** — egui pose le
      pointeur de redimensionnement, mais le lien « Registres étendus »
      touchait le bord du panneau et réclamait la main : le pointeur
      disait « cliquable » là où il fallait tirer. Une poignée qui ment
      est pire qu'une poignée muette. Deux pixels d'air le règlent — pas
      quatre, qui coupaient « Détail » à 1280x820.

## 3. L'éditeur

- [x] Fermeture automatique des paires, réglable (A 4.14)
- [x] L'autocomplétion se pilote — Tab, flèches, Échap, clic (A 4.14)
- [x] Duplication de ligne (A 4.14)
- [x] Surligner les occurrences, par jetons et non par texte (A 4.14)
- [x] Aller d'une occurrence à l'autre, sélection prête à remplacer (A 4.14)
- [x] Renommer toutes les occurrences (A 4.14)
- [x] Coller insère au curseur au lieu de tout remplacer
- [x] La gouttière montre au survol ce qu'un clic y ferait
- [x] La ligne où le programme est arrêté porte une bande
- [x] Les compteurs de vim — `5j`, `3w` (A 1.2, 7.13)
- [ ] **Le multicurseur proprement dit** (A 4.14) — vérifié dans egui
      0.36 : `TextEditState` n'a qu'un seul champ `cursor`. En tenir
      plusieurs demande de remplacer le widget de texte, ou d'écrire une
      couche qui rejoue chaque frappe à chaque position. Ce qui précède
      couvre son usage réel.

## 4. Fonctions

- [x] Patcher un binaire — inverser, forcer, neutraliser (A 4.1, 7.3)
- [x] Nommer les fonctions et les adresses (A 4.2)
- [x] Commentaires sur le listing (A 4.3)
- [x] Suivre une valeur à rebours (A 4.4, 7.5)
- [x] Points d'arrêt conditionnels sur expression (A 4.6)
- [x] Le graphe s'exporte en DOT (A 4.7)
- [x] Vue hexadécimale éditable (A 4.8)
- [x] Chercher dans la mémoire (A 4.9, 7.4)
- [x] Calculatrice multi-bases (A 4.10)
- [x] Reprendre le travail après extinction (A 4.11)
- [x] Chronologie de l'exécution (A 4.12)
- [x] Entropie dans l'onglet RE (A 4.13)
- [x] DZF branché comme désassembleur (A 4.16) — et sa limite levée le
      29 septembre : `{text}` lui donne les octets exécutables seuls,
      `{base}` leur adresse. Il ne désassemble plus l'en-tête ELF comme
      du code, sans avoir rien eu à changer. Le contrat vaut pour tout
      outil qui ne lit pas l'ELF, ce qui est le cas du premier
      désassembleur que quiconque écrit.
- [x] `keymap.json` surveillé et exportable en entier (A 4.5)
- [x] Les raccourcis, repris par famille puis par le sens (A 1.23, 7.1)
- [x] **L'onglet Octets est éditable** — devant un binaire ouvert, un
      clic sur les octets d'une instruction ouvre un champ ; Entrée
      valide, Échap annule. Même chemin que les trois gestes du
      patcheur, donc la fenêtre Correctifs les liste et sait les
      défaire. La longueur ne change pas, et c'est refusé plutôt que
      rattrapé.
- [x] **La provenance à travers la mémoire** — une valeur rangée sur la
      pile puis relue se remonte jusqu'à elle. C'est le chemin ordinaire
      de tout argument dans du code compilé sans optimisation, et la
      remontée s'y arrêtait net. Deux pistes sont suivies, non une :
      QUELLE case (les registres de l'adresse) et QUOI dedans (la
      dernière écriture). Une écriture vers une case qu'on ne sait pas
      distinguer de la nôtre coupe la piste — `[rax]` vaut peut-être
      `[rbp-8]`, et rien dans le code ne le dit.

## 4 bis. Les mots de l'interface

- [x] **Un mot par question, pour les outils** — trois questions se
      mélangeaient en autant de vocabulaires (« allumer », « s'en
      servir », « programme détecté »). Elles sont nommées séparément :
      le programme est **installé sur la machine**, il est **activé dans
      namsdec**, et le module d'installation est activé. Les trois sont
      indépendantes, et c'est tout le sujet.
- [x] **« Recommencer » retiré d'un outil qui répond** — une offre sans
      motif : vingt minutes de compilation pour un résultat identique, et
      l'on finissait par douter que l'installation ait abouti. À la place,
      **« Réparer »** dans le seul cas où relancer sert : l'outil là mais
      incomplet, c'est-à-dire rizin sans `rz-ghidra`.
- [x] **Six fiches ne tiennent pas dans la fenêtre** — trouvé par la CI,
      que ma machine cachait : elle a tout d'installé, le coureur n'a rien.
      Une fiche absente écrit une ligne de plus, et les fiches se
      dépliaient d'office — RetDec et DZF passaient sous le bord sans
      qu'aucune barre ne le dise. « Pour l'obtenir » sort du repli (c'est
      une panne, pas un détail), le repli reste fermé, et le nombre
      d'outils connus est écrit à côté du titre. Le contrôle sème
      désormais le pire cas — **rien** d'installé — et vérifie que les six
      sont atteignables en déroulant.
- [x] **DZF dans la liste**, bien qu'en cours d'écriture. Deux contrôles
      le refusaient : l'un exigeait `{file}` alors que DZF reçoit
      `{text}`, l'autre lisait le `-S` de `cmake -S .` comme celui de
      `pacman -S` et allait demander aux dépôts s'ils connaissent un
      paquet nommé « . ».
- [x] **Le dialogue qui montrait une sortie sous un titre d'entrée** —
      « Entrée du programme » gardait ce titre après la réponse. Il
      s'intitule « Ce que le programme a écrit » dès qu'il ne recueille
      plus rien.
- [x] **Le profil dit combien de lignes sont classées** — le défilement
      marchait, et c'est mesuré : douze tours de molette atteignent la
      dernière rangée. Le grief « je ne sais pas me déplacer » était de
      la découvrabilité, pas de la mécanique : rien ne disait qu'il y
      avait une suite, et l'on ne cherche pas à dérouler ce qu'on croit
      avoir vu en entier.
- [x] **Les scripts annoncent avant d'agir** — `install.sh` et
      `uninstall.sh` montrent en tête les réglages retenus, avec l'option
      qui annule chacun. On découvrait après coup qu'ils avaient touché au
      fichier du shell ou épinglé au dock. Cadre, barre d'avancement et
      bilan, et un contrôle qui vérifie l'alignement du cadre — un titre
      accentué le décalait, parce que `printf '%-*s'` compte les octets.

## 4 ter. Les contrôles qui lisaient la machine

- [x] **Quatre CI rouges pour un seul contrôle** (et non pour la mise en
      forme : `cargo fmt --all --check` passait). Il semait l'état des
      outils, mais ouvrir la fenêtre d'installation jette ce relevé par
      conception — on a pu installer l'outil à la main dans un terminal
      —, et l'image suivante le refaisait en lançant vraiment rizin :
      installé ici, absent du coureur. Il sème maintenant **après la
      première image**, et couvre les trois états au lieu d'un.
- [x] **L'onglet Outils** avait le même défaut, révélé en y ajoutant
      DZF : il lisait la machine pour savoir s'il devait vérifier quoi que
      ce soit, et ne vérifiait donc rien sur un poste bien équipé.
- [x] **Toute la classe, et non les instances** (A 3.1) — le défaut
      était dans **quatre** contrôles, pas un. Trois autres lisaient
      l'état des outils pour choisir leur assertion : le plan
      d'installation, le panneau des outils, et celui du greffon
      manquant. Ce dernier n'éprouvait **jamais** le cas « incomplet »,
      le seul qui l'intéressait, parce que rizin fonctionne sur cette
      machine. Un contrôle qui s'adapte ne signale pas qu'il a cessé de
      voir.
- [x] **La règle de A 7.6, enfin tenue** — 58 tests peignaient sans poser
      le thème, dont 11 mesuraient une géométrie sous des espacements qui
      n'existent nulle part. Les 11 posent le style, `paint` le pose pour
      tous, et `a_control_that_measures_geometry_sets_the_style_first`
      refuse qu'un test lise une position, une taille de galley ou une
      forme peinte sans l'avoir posé. Les 47 autres ne cherchent qu'un mot
      : l'exiger d'eux serait du bruit, et la règle le dit.
      Relecture faite aussi des 44 lectures de `.pos` : elles combinent
      presque toutes position et taille, donc un rectangle. Rien à
      réécrire — c'est un résultat, pas un renoncement.
- [x] **`no_control_asks_the_machine_what_it_should_be_checking`** — le
      contrôle qui empêche la récidive. Il lit `tests.rs`, découpe par
      `#[test]`, et refuse `availability_in`, `.availability()` et
      `tool_availability` dans un corps de test. Deux exceptions nommées
      avec leur raison — vérifier une relation vraie partout, vérifier la
      mécanique du relevé —, et la liste blanche est elle-même vérifiée :
      un nom qu'on y laisse après avoir renommé le test ferait croire à
      une exception qui n'existe plus. Éprouvé en remettant le défaut.
- [x] **Le guide que le README annonce** — le lien pointait vers un PDF
      que trois montées de version avaient emporté. Un contrôle exige
      maintenant qu'il porte la version courante, que le fichier existe,
      et qu'aucun autre ne traîne à côté.

## 4 quater. La mise en page du listing, mesurée

- [x] **Les 280 ms, chiffrées et rangées** (A 5.3) — sur 15 000 lignes :
      coloration 40 ms, **pose des glyphes 285 ms**, total 325. La pose est
      87 % du coût. Sur le crackme (62 386 lignes) : première image 561 ms,
      puis 2,4 ms en régime, et **une seule pose** pour tout le chargement.
      Le cache fait son travail ; le prix est celui d'un chargement.
- [x] **`opening_a_binary_lays_out_its_listing_once`** — retient le « une
      fois ». Une régression qui reposerait à chaque image ne se verrait pas
      sur un source de vingt lignes et rendrait l'application inutilisable
      sur un binaire : une demi-seconde par image.
- [x] **Piste fermée : fusionner les sections de même format.** Aucun gain
      — `egui::LayoutJob::append` le fait déjà (epaint 0.36.2). Les 120 000
      sections mesurées sont déjà le résultat de sa fusion, huit par ligne
      au lieu de dix. Le refaire coûtait 22 ms de plus. Retiré, et le
      commentaire de `append` le dit pour la prochaine fois.
- [x] **Ce qui restait — ne poser que les lignes visibles — est fait**
      (A 7.14, section 8) : le 14 octobre 2026, en mode Normal, où le
      champ de texte était déjà non interactif. Ce que cette entrée
      craignait de perdre (sélection, recherche, curseur, marques)
      n'existait pas dans ce mode-là.
- [x] **Et ce qui se payait à CHAQUE image, lui, est corrigé** — voir la
      section 7 : le régime sur un listing d'un million de lignes passe de
      11,3 ms à moins de 2 ms. Le relevé de 5.2 ne l'avait pas vu parce
      qu'il ne mesurait que crackme-01, le plus petit binaire du dossier
      d'épreuve.

## 4 sexies. Robustesse sur entrée hostile

- [x] **Panique corrigée dans `imports::scan`** — un fuzzing (40 000 ELF
      malformés) a trouvé deux paniques : `sym * sym_entry` débordait,
      `sym` venant d'un champ quelconque du fichier. Corrigé par
      arithmétique vérifiée, comme le reste du module. C'est la première
      exigence d'un outil qui reçoit des fichiers malveillants : ils sont
      délibérément malformés. Invariant tenu par
      `a_hostile_elf_never_panics_the_scan`. Reconfirmé : 0/40 000.
- [x] **L'invariant étendu à TOUS les analyseurs** (C2) — un contrôle
      d'intégration, `tests/hostile.rs`, fait passer une vingtaine
      d'analyseurs, le désassembleur et les deux rapports sur des ELF
      abîmés de trois façons : octets changés, troncature, et valeurs
      extrêmes posées sur huit octets alignés. Le germe est **fabriqué**
      — en-tête, segments, sections, symboles, relocations, entrées de
      liaison — et non lu sur le disque : le contrôle d'origine partait
      de `/bin/true` et passait sans rien prouver sur une machine qui
      n'en a pas.
      **Quatre paniques trouvées et corrigées** : `imports::file_offset`
      (deux sommes d'adresses), `slack` (le nom de section et l'adresse
      virtuelle d'un trou), et surtout les **trois lecteurs** `u16_at`,
      `u32_at`, `u64_at` d'`elf_scan` et d'`imports` — `at + 4` déborde
      quand `at` vient d'un champ du fichier, et toutes les additions de
      décalage de champ sont désormais saturantes. C'était une classe de
      défauts, non un défaut.

## 4 septies. Lire un échantillon : ce qui ressort, et ce qu'on en recopie

La séance du 1er octobre, menée sur le relevé `septembre26.md`. Tout
s'appuie sur du déjà-calculé : aucun de ces panneaux ne relit le fichier
pour son compte.

- [x] **Les familles de comportement** (A1) — `behaviour.rs` range les
      noms en six familles : réseau, exécution, anti-débogage, mémoire,
      chiffrement, fichiers. Une seule table pour les imports **et** les
      appels système, parce qu'un `socket` est un `socket` qu'il vienne
      de la table de liaison ou du numéro 41 dans RAX — et qu'un
      programme qui veut passer inaperçu emploie justement le second.
      Une table de noms exacts et non des règles : « tout ce qui commence
      par `exec` » attraperait `execute_query`. Le portrait se dessine en
      tête des deux panneaux, avec les mêmes couleurs.
      Ce qui n'y est **pas** : `raise`, `alarm`, `sigaltstack`,
      `rt_sigaction`. La capture d'écran a montré `/bin/ls` allumant
      « Anti-débogage » sur `raise` — un tableau de bord qui crie sur `ls`
      n'est plus lu.
- [x] **Le panneau « Signaux »** (A2) — la synthèse, en seconde position
      dans la colonne. Il n'analyse rien de neuf : empaqueteur reconnu,
      segment W+X, entropie haute sur une **grande** tranche, protections
      absentes, familles présentes, code auto-modifiant possible,
      indicateurs. Chaque ligne dit un fait, ce que ce fait change, et le
      panneau qui l'explique. **Rien n'est additionné en score** : un
      score donnerait l'illusion d'un verdict.
      Un contrôle garde le panneau utilisable : sur `/bin/true` et
      `/bin/ls`, aucune ligne « Remarquable ».
      La tranche d'entropie y est mesurée sur 4 Kio au moins, et non sur
      les soixante-quatrièmes du panneau Entropie : la queue de
      `/bin/true` fait 505 octets et mesure 7,61 sans rien cacher.
- [x] **Les indicateurs dans les chaînes** (A3) — `ioc.rs` : adresses
      web, adresses IP, noms de domaine, courriels, chemins absolus, clés
      de registre. Ce sont les seules choses d'un binaire qu'on puisse
      aller vérifier **ailleurs**. La règle est écrite pour se taire
      plutôt que se tromper : une liste de domaines de premier niveau
      connus, et non « tout ce qui contient un point » — sans quoi
      `libc.so.6` passerait pour un domaine et la liste serait
      inutilisable. Chaque rubrique se copie d'un clic.
- [x] **L'empaqueteur nommé** (A4) — `packer.rs` : UPX, PyInstaller,
      AppImage, par nom de section ou nombre magique, avec **la preuve**
      et ce qu'on en fait (`upx -d`, `pyinstxtractor`). Le nombre magique
      d'AppImage n'est cherché qu'à sa place, au décalage 8 : trois
      octets se rencontrent par hasard partout.
- [x] **Les seuils d'entropie rassemblés** (C3) — `entropy::seuils`, avec
      la mesure qui justifie chacun. Ils étaient écrits en clair dans un
      `match` et recopiés dans le panneau.
- [x] **Le rapport porte tout cela** — signaux, portrait, familles sur
      chaque import, indicateurs par nature. C'est la **même** fonction
      qui produit les signaux du rapport et ceux du panneau.
- [x] **Le relevé de ce qui est peint** — `tests/panneaux.rs` mesure les
      cinq panneaux touchés, à deux largeurs et dans les deux langues :
      rien ne court au-delà d'un bord, rien ne se recouvre, rien ne bouge
      tout seul, et aucune phrase n'est trouée d'espaces. Ce dernier
      contrôle vient d'un vrai défaut : un littéral Rust coupé sur
      plusieurs lignes garde ses espaces d'indentation quand le `\` de
      continuation disparaît.
- [x] **Le désalignement volontaire** (A5) — vérifié d'abord, mesuré :
      le désassemblage de namsdec est **linéaire**, et c'est le bon choix
      pour un listing. Sur la figure classique (`jmp $+3` / `db 0xE8` /
      `xor eax,eax`), il rend `call 0FFFF…h` puis `(bad)` là où le
      processeur lit `xor eax,eax ; inc rax ; ret` — et le graphe de flot
      perdait l'arête sans rien dire.
      `misalign.rs` relève les endroits où un saut atterrit **au milieu**
      d'une instruction du listing, et le panneau « Figures du code »
      donne les **deux lectures côte à côte**. Il ne corrige pas le
      listing : un listing troué serait pire que trompeur, et nommer
      l'endroit suffit à ne plus se faire avoir.
      Deux bornes, chacune venue d'une mesure sur `/bin/ls` : seules les
      sections **exécutables** comptent (une chaîne de format décodée
      comme du code rend des sauts qui tombent au hasard), et le saut doit
      tomber à moins de seize octets (la figure cache un ou deux octets,
      pas soixante). Sans elles, cinq figures relevées sur `/bin/ls`, et
      aucune vraie.
      Relevé aussi dans les signaux et dans le rapport, avec les deux
      lectures.

## 4 octies. Circuler : d'un panneau d'analyse au listing

- [x] **Le compte d'appels des imports, réparé** — le panneau annonçait
      105 fonctions importées et **zéro** appel vers chacune sur un
      binaire ouvert. Le comptage convertissait l'adresse d'une entrée de
      liaison en position dans le fichier, parce que le commentaire
      d'`imports_module` dit que « le désassemblage part du premier octet
      du fichier ». C'est vrai d'un tampon brut et **faux** d'un ELF
      ouvert : `load_elf` ne désassemble que les parties exécutables,
      chacune à son adresse virtuelle. Les deux conventions sont essayées
      maintenant ; mesuré sur `/bin/ls` : 719 appels au lieu de 0.
      Un compte faux se lit comme un compte vrai : c'est pourquoi le
      défaut a tenu. Le contrôle qui le retient charge le binaire **par
      l'application**, et non en reconstruisant le listing à sa façon.
- [x] **Les adresses des panneaux sont cliquables** — une entrée de
      liaison, une zone libre, une figure, un fragment, un désalignement :
      cliquer l'adresse amène le listing dessus, par
      `ReRequest::GoTo`. Le suivi de RIP s'arrête au passage, sans quoi
      le pas suivant ramènerait le listing là où l'exécution en est.
      Une adresse entre deux lignes mène à la ligne juste avant — c'est
      le cas d'un désalignement volontaire ; une adresse dans aucune
      partie désassemblée le dit plutôt que de ne rien faire.
      Le contrôle **clique pour de vrai** : il peint le panneau, cherche
      où l'adresse a été posée, et envoie un appui puis un relâchement.
      Vérifier que le code « poserait une demande si on cliquait » ne
      prouverait pas qu'on peut cliquer.

## 4 nonies. Les raccourcis, et ce que le banc ne regardait pas (A 7.1)

- [x] **Les deux jeux côte à côte** — l'onglet n'en montrait qu'un, et
      c'est précisément ce qui empêchait de voir les trous et les
      doublons : on ne compare pas de mémoire deux listes de trente
      lignes. Le jeu actif en clair, l'autre à côté ; « la même » quand
      les deux disent la même chose, parce que c'est ce qui DIFFÈRE qu'on
      vient lire ; un raccourci déplacé soi-même est marqué et dit au
      survol ce que le jeu proposait ; deux actions qui répondent à la
      même frappe se voient en rouge.
      Le rangement par famille (`Espace p…`, `Espace v…`) était déjà fait
      ; c'est ce tableau qui manquait pour en juger.
- [x] **Les séquences s'écrivent `Space P I`** et non `Space , P , I`.
      Les deux formes côte à côte sur trente lignes ont tranché : la
      virgule entre deux espaces n'ajoute rien. Le projet écrit d'ailleurs
      « Espace p i » partout dans sa prose.
- [x] **Les cinq pages des préférences entrent au banc** — seule la
      première y était, la seule qui tienne sans défiler. Les quatre
      autres n'avaient jamais été mesurées, et deux défauts y
      dormaient.
- [x] **La boîte de préférences grandissait d'un pixel par image, sans
      fin** — défaut antérieur, trouvé par le banc dès que les autres
      pages y sont entrées. La zone qui défile prenait sa hauteur sur la
      fenêtre ; la fenêtre prenait la sienne sur son contenu. Trois
      essais : borner la zone sur le viewport coupe la boucle mais fait
      sortir « OK » par le bas ; borner la fenêtre par le haut laisse
      deux cent quarante images de reptation ; **imposer** la hauteur de
      la fenêtre répond aux deux. Un contrôle garde la cause, page par
      page, et il échoue sur l'ancien code.
- [x] **Un contrôle qui lisait l'horloge** — « ouvrir un binaire ne pose
      son listing qu'une fois » gardait un budget de 40 ms par image. Il
      a échoué deux fois sous une suite complète alors qu'il passait
      seul : une machine chargée mesure mal. Il compare maintenant un
      RAPPORT — les images d'après coûtent moins de la moitié de la
      première — ce qui ne dépend plus de la charge, les deux mesures la
      subissant ensemble. Le compte de poses, lui, était déjà
      déterministe et ne change pas.
- [x] **Une phrase pour deux rôles** — « Aucun binaire assemblé. » était
      à la fois l'état vide d'un panneau (un bloc centré) et l'état d'une
      fiche de module (une ligne à gauche). C'est la même maladie que
      deux formes pour un rôle. La fiche dit maintenant « rien à
      désassembler pour l'instant ».

## 4 quinquies. Modifier un binaire — premier morceau

Fred a rappelé la vocation : l'onglet RE apprend la rétro-ingénierie,
comme Ghidra ou un CTF. La fonctionnalité annoncée — modifier le binaire
ouvert pour y ajouter ou retirer quelque chose — a un mur : tout ce que
`patch.rs` sait faire garde la longueur exacte, donc on peut retirer,
jamais ajouter. Il faut de la place, et d'abord savoir où elle est.

- [x] **`slack` : la place libre d'un binaire** — lecture seule. Parcourt
      les segments chargeables et repère les longues suites d'un même
      octet de bourrage (0x00, 0x90 nop, 0xcc int3) que l'éditeur de liens
      laisse pour aligner. Robuste sur un binaire dépouillé : le bourrage
      se reconnaît à lui-même, sans en-têtes de section. Borné par
      `filesz`, jamais `memsz` — `.bss` n'occupe rien sur le disque.
- [x] **Le panneau `slack`** — dans la grappe pédagogique, juste après les
      fragments ROP : là où ceux-ci montrent qu'un binaire contient déjà du
      code réutilisable, celui-ci montre qu'il contient de la place
      inutilisée. L'octet de bourrage est NOMMÉ (« 0xcc (int3) ») et non
      seulement chiffré. Vérifié sur `/bin/true` et `/bin/ls` : 1 906 et
      6 280 octets libres.
- [x] **La section qui porte chaque zone** — `.text`, `.plt.sec`, `.rodata`…
      C'est elle qui raconte l'histoire : 50 octets libres dans `.plt.sec`
      n'est pas 245 en fin de `.rodata`. `None` sur un binaire dépouillé ou
      pour un trou entre deux sections. `.bss` (qui n'occupe rien) ne nomme
      jamais un trou, bien que son adresse chevauche celle d'une vraie
      section.
- [ ] **La suite, non commencée** : assembler du code dans une zone libre,
      et détourner l'exécution vers lui puis la ramener. Passe par
      `patch.rs`, qui sait déjà défaire ce qu'il pose, et par
      `ReRequest::PatchBytes`, qui existe déjà. À faire pas à pas, chaque
      geste réversible et montré avant d'être posé — comme les correctifs.

## 4 sexdecies. DZF : namsdec sait l'installer lui-même

- [x] **Recette d'installation de DZF** (7 octobre 2026) — DZF vit dans
      son propre dépôt (`github.com/fredza/DZF`), jamais en dépendance.
      `recipe_for("dzf")` le clone, le configure et le construit en
      C++23 avec `cmake` (`DZF_BUILD_TESTS=OFF`), puis copie
      `x64-disassembler` et `dzf-plugin-engine` dans `<tools>/dzf/bin/` —
      `cmake` n'installe que la bibliothèque, pas les exécutables. Rien
      hors du dossier de namsdec, aucun `sudo`, chaque commande montrée
      avant d'être lancée. Même patron que Rizin et RetDec.
- [x] **Le bouton Installer… apparaît pour DZF** — dès qu'une recette
      existe (`recette_existe`), la fiche de DZF porte le bouton, comme
      les autres. La chaîne d'installation manuelle est corrigée (vrai
      dépôt, bon chemin de build), `dzf.json` régénéré, manuel et README
      des fonctionnalités à jour.

## 4 septdecies. L'onglet RE : graphe d'appels, aller-à, code et pseudo-code

Fred a demandé trois choses pour l'onglet RE : aller à une adresse, voir
les appels de fonctions en graphe (style UML), et des rubriques montrant
le code binaire et le pseudo-code. Un seul module neuf les réunit.

- [x] **Module « Graphe d'appels »** (`call_graph_module.rs`) — greffé
      juste après « Fonctions », dont il prolonge le travail :
      `functions::discover` donne déjà `calls`, c'est-à-dire les arêtes du
      graphe. Rien de neuf à calculer.
- [x] **L'arbre des appels, à la façon UML** — déplié depuis le point
      d'entrée (ou `main`), une boîte cliquable par fonction, un trait de
      liaison peint entre parent et enfant. Une fonction déjà dépliée
      devient un renvoi (`↩`) : l'arbre reste fini même sur du code
      récursif. Borné (40 fonctions, profondeur 8, 120 nœuds) — au-delà,
      un graphe n'est plus une carte.
- [x] **Aller à une adresse** — un champ qui lit l'hexadécimal (`0x…`) ou
      le décimal, et émet `ReRequest::GoTo` ; chaque boîte de l'arbre est
      aussi un raccourci. S'appuie sur le canal `GoTo` qui existait déjà.
- [x] **Rubriques code binaire + pseudo-code** — sous l'arbre, chaque
      fonction se déplie sur ses octets bruts (le code binaire) et leur
      traduction en clair (le pseudo-code, moteur `pseudo::of`), côte à
      côte. Instructions prises dans la plage de la fonction par recherche
      dichotomique, bornées à 200.
- [x] **Vérifié** — trois tests du module (lecture d'adresse, arbre fini
      sur récursion, instructions bornées à la plage) ; les invariants du
      banc (non-recouvrement, découpe, contraste) couvrent le panneau via
      la scène « Analyse ».

### Retour de Fred (7 octobre, après essai) — corrigé

- [x] **Le graphe se voyait comme une liste, pas comme un graphe.** La
      première version posait des boutons empilés et traçait des traits
      par-dessus ; aux couleurs discrètes du thème (fond `faint_bg`,
      traits `weak_text`), rien ne se lisait comme un diagramme. Réécrit
      en vrai canevas peint (`allocate_painter`, comme le déroulement CFG
      de `flow.rs`) : boîtes à fond et cadre francs, connecteurs fléchés
      parent→enfant, racine/`main` en accent. Clic sur une boîte → on y
      va. Mesuré : le panneau peint bien des boîtes, et les invariants du
      banc le couvrent.
- [x] **Cliquer une référence croisée ne menait nulle part.** Le module
      `xrefs` ne faisait que déplier les appelants — il n'émettait aucun
      `GoTo`. Désormais l'adresse est cliquable (y aller), le triangle
      déplie (d'où c'est appelé), et les adresses « appelé depuis » et
      « bouclée depuis » mènent aussi. `take_request` branché.
- [x] **« En bref » manquait de détails.** Le résumé dit maintenant ce
      que le binaire **emprunte** (fonctions + appels système), **ce
      qu'il sait faire** — le portrait en six familles, dont le réseau :
      la réponse à « a-t-il accès à Internet ? » se lit d'un coup d'œil,
      qu'il passe par la table de liaison ou par un numéro d'appel
      système —, et les **protections** qui décident de tout (PIE, NX,
      RELRO, canari), cochées ou barrées. Tout mesuré, rien déclaré.

- [x] **Renommer les fonctions** (demandé par Fred) — chaque fonction,
      dans le détail du graphe d'appels, porte un champ **Nom** : on
      remplace `sub_10450` par « vérifie_clé », « Défaut » rend le nom
      d'origine. Pas une table neuve : ça écrit dans les **notes** du
      binaire (`notes.names`, `ReRequest::Rename` → `set_name` + `save`),
      la même que le listing et le déroulement emploient déjà. Le nom se
      lit donc partout — arbre, liste des fonctions, listing — et il est
      **gardé d'une séance à l'autre** (persisté par empreinte du
      binaire). `ReContext` porte la table (`names`), les 29 littéraux
      complétés. Un test : un nom donné prend la place du défaut.
- [ ] **Reste demandé par Fred, non commencé :** un **éditeur binaire**
      (modifier les octets à la main dans l'onglet RE — passe par
      `patch.rs` et `ReRequest::PatchBytes`, pas à pas et réversible, voir
      « Modifier un binaire »).

## 4 octies. Découpage de main.rs (suite)

- [x] **main.rs : 6 643 → 4 231 lignes** (−2 412, −36 %), sans rien
      changer au comportement. Neuf modules neufs, chacun un thème net,
      chacun vérifié par compilation puis par la suite complète :
      - `icons.rs` (217) — les dix icônes dessinées à la main (peintre,
        rectangle, couleur ; dessins purs, testés au pixel).
      - `widgets.rs` (667) — barres de validation et de dialogue, boutons
        (texte, action, icône), cartes de thème et de module, `Answer`.
      - `labels.rs` (104) — les étiquettes du source (fonctions pures sur
        du texte, sans écran).
      - `resources.rs` (169) — dossiers de données et ressources
        installées (greffons, thèmes), et les deux libellés de région.
      - `profiling.rs` (82) — le diagnostic des images lentes : mesurer
        chaque étape d'une frame, et écrire celle qui a gelé l'interface.
      - `listing.rs` (~280) — le rendu du listing désassemblé : bande de
        chaleur, ligne sous le curseur, boutons-liens, étiquettes de
        liaison, annotation d'une ligne, cible d'un saut, nom d'adresse.
      - `enums.rs` (~530) — les onze énumérations d'état de l'interface
        (onglets, modes, formats, statut) et leurs libellés.
      - `patching.rs` (~285) — les méthodes de `NamsdecApp` qui modifient
        le binaire : prévoir un correctif, l'écrire (octets ou instruction
        assemblée), le défaire. Premier groupe de MÉTHODES extrait, sur le
        patron des modules frères (`impl NamsdecApp` + `use super::*`).
      - `exports.rs` (~155) — le texte que les vues produisent pour la
        copie ou l'enregistrement : contexte d'une ligne, listing entier,
        rapport d'analyse.
- [x] Les neuf ajoutés à `SOURCES_DE_L_INTERFACE` : les contrôles qui
      lisent le code source les voient désormais comme les autres.
- [x] **Reprise du 6 octobre : les méthodes des outils vers `tools.rs`**
      (−96 lignes, 4 272 → 4 176). `refresh_tool_availability`,
      `tool_availability`, `declare_tool`, `undeclare_tool` rejoignent le
      module où vit déjà toute la gestion des outils extérieurs. Même
      patron (`impl NamsdecApp` + `use super::*`), passées `pub(crate)`
      pour les frères qui les appellent (jobs, tests). Comportement
      inchangé, suite complète au vert.
- [x] **Puis la géométrie de la console vers `console.rs`** (−118,
      4 176 → 4 058). `console_wanted_height` et `console_input_estimate`
      — la hauteur que le panneau du bas et sa zone de saisie réclament —
      rejoignent le module de la console. Même patron, `pub(crate)` pour
      `draw` qui les appelle.
- [x] **Et l'entrée de la console : `console_shows_input` + `send_input`**
      (−46, 4 058 → 4 012). Donner au programme la ligne saisie (ou fermer
      son entrée), et dire si le champ doit paraître : les deux rejoignent
      `console.rs`. (`send_input` ne faisait que ~20 lignes, non ~440 — une
      borne mal lue la séance d'avant, corrigée ici.) La console tient
      désormais toute sa logique dans son module.
- [x] **Le cycle des préférences vers `preferences.rs`** (−41,
      4 012 → 3 971, main.rs passe sous 4 000). `open_preferences`
      (mémorise l'état des réglages pour que « Quitter » ait où revenir) et
      `close_preferences` (OK garde, Quitter restaure) rejoignent le module
      des préférences. `pub(crate)` pour menus et palette qui les appellent.
- [x] **Le curseur de l'éditeur vers `editor.rs`** (−56, 3 971 → 3 915).
      `current_line` (la ligne où le programme est arrêté), `apply_goto_line`
      (poser le curseur au début d'une ligne et y amener la vue) et
      `apply_pending_cursor` (replacer le curseur après une modification)
      rejoignent le module de l'éditeur. `pub(crate)` pour la barre de
      statut et l'éditeur qui les appellent.

## 4 nonies. Toutes les instructions nasm, visibles

- [x] **Le sommaire de l'aide montrait 113 têtes de famille pour 674
      mnémoniques** — `jmp` se voyait, mais `je`/`jne`/`jz`… étaient dans
      la doc sans être à l'écran : 561 instructions nasm qu'il fallait
      connaître pour les chercher. Trouvé sur les captures de Fred (2-3
      octobre). Le sommaire des instructions affiche désormais **tous les
      noms** de chaque fiche, chacun cliquable (nom surligné à
      l'ouverture), avec un compte en tête. Contrôle :
      `the_instruction_index_shows_every_mnemonic_not_just_the_family_head`.
      Manuel et PDF à jour.

## 4 quindecies. Gestion plugins/modules : première passe

- [x] **Onglet Modules clarifié** — le mot « module » couvrait deux choses :
      les COMPOSANTS (crates, santé, non allumables) et les FONCTIONS
      optionnelles (RE, exercices, décompilation, installation). Deux
      sous-titres les séparent désormais.
- [x] **Carte d'outil incomplet corrigée** — pour rizin « installé, mais
      incomplet », « Pour l'obtenir : sudo dnf install rizin » se
      contredisait (la base est là) et visait le mauvais paquet (ce qui
      manque, rz-ghidra, n'est pas un paquet dnf). La ligne n'apparaît plus
      que si l'outil est ABSENT ; pour l'incomplet, « il lui manque… » + le
      module d'installation (qui construit ce qui manque) suffisent.
      Contrôle : assertion ajoutée à `a_working_tool_is_not_announced…`.
- [x] **La décompilation éteinte est signalée dans l'onglet Outils** — les
      deux réglages d'une même fonction tombaient sur deux onglets (activer
      un outil dans Outils, allumer la décompilation dans Modules). L'onglet
      Outils le dit en tête quand elle est éteinte, avec un bouton pour
      l'allumer sur place. Contrôle : `the_tools_tab_says_when_decompilation_is_off`.
- [x] **Localiser un binaire bâti hors du PATH** — DZF compilé dans son dépôt
      restait « absent » : namsdec ne cherche que dans le PATH et son dossier
      d'outils, et un binaire laissé dans `DZF/x64-disassembler/build/` n'est
      ni l'un ni l'autre. La fiche d'un outil absent propose maintenant
      **Localiser le binaire…** : on désigne le fichier une fois, il est
      retenu (dans `settings.json`, clé `located_tools`), employé pour la
      détection (prêt si le fichier existe) et comme commande à la déclaration.
      Vaut pour tout outil rangé hors des chemins habituels, pas que DZF.
      Contrôle : `a_located_binary_makes_a_tool_ready`.
- [x] **L'éditeur suit l'exécution** — si l'on avait défilé vers la droite
      (pour lire le bout d'une longue ligne de commentaire), un pas gardait
      cette vue : ligne courante visible en hauteur, mais instructions et
      gouttière sorties à gauche, on ne voyait plus que les commentaires.
      Chaque pas remet le défilement horizontal à zéro, tenu quelques images
      pour absorber l'élan d'une molette lancée. Vu en vidéo (exj4.asm).
      Contrôle : `stepping_brings_the_code_back_into_view`.
- [x] **Les flags redeviennent de simples indicateurs** — ils étaient
      cliquables (un clic basculait l'indicateur quand un programme
      tournait), ce qui brouillait leur lecture et n'était possible que par
      intermittence. Désormais on les lit, on ne les règle plus : `Sense`
      au survol (l'infobulle explique), plus de clic. `toggle_flag` et la
      note « Indicateur basculé à la main » retirés ; le test
      `a_register_is_set_by_hand` ne couvre plus que le registre.
- [x] **Enchaînement Activer / Réparer d'un outil incomplet** — activer un
      outil incomplet (rizin sans rz-ghidra) déclarait un décompilateur qui
      échoue au premier essai (`pdg` n'existe pas). « Activer » est désormais
      **grisé** tant que l'outil n'est pas prêt (infobulle qui l'explique),
      et son bouton de complétion devient **« Réparer »** en accent (et non
      « Installer… », mot d'un outil absent). On ne peut activer qu'un outil
      prêt. Contrôle : `an_incomplete_tool_points_to_repair_not_enable`.
- [x] **Revue plugins/modules close** : frontières nettes (Composants/
      Fonctions sous Modules, Thèmes/Exercices sous Plugins, programmes sous
      Outils), fiche d'outil incomplet corrigée, décompilation éteinte
      signalée, binaire hors PATH localisable, et l'enchaînement
      Activer/Réparer réglé.

## 4 quaterdecies. Flags toujours affichés, éclat tenu plus longtemps

- [x] **Les sept flags s'affichent en permanence** dans la barre de titre,
      même avant d'assembler (tous à zéro, grisés, non cliquables tant
      qu'il n'y a pas d'émulateur). La barre restait muette au repos ;
      « tout doit s'afficher ».
- [x] **L'éclat orange dure ~3 s avec un maintien** — il reste à pleine
      couleur ~45 % du temps puis fond, au lieu d'une décroissance dès le
      premier instant. « Clignoter longuement. » `FLASH_SECONDS` 1,5 → 3,0,
      `FLASH_HOLD` = 0.45.

## 4 terdecies. Flags à droite du nom, et éclat plus long

- [x] **Les indicateurs à droite du nom du fichier** (et non à gauche) —
      au bout de la barre de titre, via une disposition droite-à-gauche.
      egui inversait alors l'ordre des pastilles ; on parcourt FLAG_BITS à
      l'envers pour que le double renversement redonne CF…OF. Mesuré :
      nom à ~470, pastilles de ~1110 à ~1358 sur une fenêtre de 1400.
- [x] **L'éclat orange dure 1,5 s** (au lieu de 0,6) — trop court, disait
      Fred. `FLASH_SECONDS` porté à 1.5.

## 4 duodecies. Les flags dans la barre de titre, le changement bien visible

- [x] **Les sept indicateurs déménagent dans la barre de titre** — c'est
      là que Fred les voulait (par « barre de titre »), pas sous un en-tête
      dans le panneau. La barre de titre est toujours visible et ne se
      replie jamais : on les a sous les yeux à chaque pas, à côté du nom du
      fichier. Clic pour basculer, survol qui développe nom/rôle/état,
      clignotement conservés. Retirés du panneau d'état (qui en ressort
      plus court — le repli libère ~81 px, seuil du test ajusté).
- [x] **Le changement d'un registre, bien plus visible** — un fond orange
      qui s'estompe derrière la valeur, à la place de la seule teinte de
      texte (trop discrète, dit Fred). Un bloc de couleur se voit d'un coup
      d'œil ; le texte reste clair dessus pour rester lisible pendant
      l'éclat. Manuel et PDF à jour.

## 4 undecies. Les indicateurs : une barre de titre, et chacun se présente

- [x] **Barre de titre « Indicateurs »** devant la rangée des pastilles —
      elle flottait sous les registres sans dire ce qu'elle était. Le titre
      et les sept pastilles tiennent sur une seule rangée : l'espacement
      resserré (4 px) et des pastilles compactes (padding 6×2) laissent la
      place, sans que l'état du processeur ne défile jamais — mesuré de
      800×520 à 1920×1230.
- [x] **Chaque indicateur se présente en entier au survol** — nom développé
      (CF → retenue, ZF → zéro, DF → direction…), rôle, et état posé ou
      retombé. Le survol ne disait que « cliquez pour basculer » ; on lisait
      `CF` sans savoir ce que c'était. `flag_toggle_hint` fondu dedans.
      Contrôle : `the_flags_carry_a_title_and_describe_each_state`.

## 4 decies. Les indicateurs : tous présents, et qui clignotent

- [x] **DF ajouté aux indicateurs** — le bandeau n'en montrait que six
      (CF PF AF ZF SF OF). DF manquait, alors que `std`/`cld` le posent et
      les instructions de chaîne (`movsb`, `stosb`…) le lisent. Ordre du
      processeur conservé : CF PF AF ZF SF DF OF. « Tout doit s'afficher. »
- [x] **Clignotement orange à chaque changement** — un registre ou un
      indicateur qui bascule s'éclaire en orange, puis retombe en 0,6 s.
      Une valeur qui change sans bouger à l'écran se ratait ; l'éclat la
      désigne sans peser sur la lecture d'après. L'instant du changement
      est posé en un point unique (`notify_step`), la couleur fond de
      l'orange vers la normale (`flash_fraction`), et l'on redessine tant
      qu'elle brille. Appliqué aux registres (généraux et XMM) et aux
      indicateurs. Contrôles : `the_flash_fades_over_real_time`,
      `every_flag_is_shown_including_df`, `a_changed_value_flashes_orange`.
      Manuel et PDF à jour.

## 5. Vocation pédagogique

- [x] **L'Atelier, seconde série d'exercices** — dix-huit exercices de
      plus, livrés en greffon (`plugins/exercices/atelier.json`) et posés
      par le même `./install.sh` : huit au palier Structures (matrice,
      longueur d'une chaîne, tri par insertion, récursion, cadre de pile,
      liste chaînée, recherche d'octet, table de sauts), dix au palier
      Maîtrise (`paddd`, `pcmpeqb`/`pmovmskb`, appel système, débordement
      signé contre non signé, produit sur 128 bits, division par
      décalage, ordre des octets, compte de bits, canari de pile, code
      auto-modifiant). Les dix-huit **atteignent leur but tels qu'ils
      sont livrés** — un contrôle les assemble et les exécute, et il a
      déjà attrapé deux énoncés faux de ma main. Les budgets sont
      MESURÉS, non devinés. Le JSON se régénère depuis
      `plugins/exercices/ecrire_atelier.py` : on corrige le Python, pas
      le JSON.
- [x] **Les échecs des exercices, expliqués** (A 7.10) — la seconde
      moitié est faite : sous un but manqué, namsdec dit ce que le
      programme a FAIT — « la boucle de la ligne 6 a tourné 4 fois »,
      « le saut de la ligne 9 n'a jamais été pris ». Des compteurs, non
      des déductions ; un saut dont la suite a d'autres entrées ne dit
      rien du tout.
- [x] **Un mode « explique-moi cette fonction »** (A 7.9) — les deux
      crans sont faits. Le bloc : cinq reconnaisseurs, chacun certain ou
      muet — appel système dont le numéro est dans le bloc, boucle avec
      son nombre de tours, comparaison suivie de son saut, bloc de
      `nop`, préparation de constantes. La FONCTION : un portrait en une
      ligne — les arguments lus avant d'être écrits, la valeur rendue
      quand tous les retours s'accordent, les appels système. Il se tait
      quand il n'aurait que ce que le dessin montre déjà.
- [x] **Les sauts rendus à leur forme** (A 7.9) — `namsdec-re/structure`
      lit le graphe d'une fonction et en écrit le pseudo-code indenté :
      `si` / `sinon` pour deux branches qui se rejoignent, `tant que`
      quand le test est en haut, `répéter … tant que` quand il est en
      bas. La condition affichée est celle du PROGRAMME, non celle du
      saut. Ce qui ne se reconnaît pas garde son « aller à ». La case
      « Pseudo-code » de la fenêtre du déroulement bascule entre les
      deux vues, et le choix part dans `settings.json`.

## 6. Tests et robustesse

- [x] **L'intégration continue passe sur le coureur GitHub** — verte
      pour la première fois le 29 septembre 2026. Elle échouait sur un
      contrôle qui lit les binaires de la machine : seize instructions
      présentes sur Ubuntu et absentes ici manquaient au manuel. Ce
      contrôle est le seul du dépôt à lire la machine sciemment, parce
      que ce qu'il trouve n'est jamais un faux positif — un échec y est
      une liste de fiches à écrire, non un défaut de l'application.

- [~] **Des tests dépendent encore de la machine** (A 3.1) — un dossier
      `tests/binaires/` existe, le reste passe encore par nasm et ld.
      Deux fuites trouvées et bouchées le 25-26 septembre : le
      `keymap.json` de l'utilisateur, lu par dix-sept tests, et la liste
      des outils installés, lue par le test du plan d'installation —
      lequel a échoué le jour où namsdec a été installé sur la machine.
- [x] `the_assembled_program_can_leave_the_workbench` stabilisé (A 3.4)
- [x] Le rendu hors écran est mesuré, et les captures sont outillées (A 3.2)
- [~] **Un contrôle qui dit ce qui n'est pas testé** (A 7.6) — commencé ;
      reste à relire les tests d'affichage existants
- [x] **Un test de bout en bout sur l'installation** (A 3.3) — il LANCE
      `install.sh`, regarde ce qui est posé sur le disque, puis lance
      `uninstall.sh` et vérifie ce qui part. Il n'écrit rien dans le
      profil : `--prefix` et `--bin-dir` l'envoient dans un dossier
      temporaire, `--no-dock` et `--no-path` lui interdisent le bureau
      et le shell, `--no-build` évite de rappeler cargo depuis cargo.
      Ces options n'existaient pas — c'est pour cela que ce test est
      resté ouvert des semaines.
- [x] **`egui_kittest` évalué, et gardé pour une seule chose** (A 3.5,
      7.7) — le mandat était « si elle ne raccourcit pas visiblement,
      elle sort ». Le même contrôle écrit des deux façons fait la même
      longueur : sur ce critère, elle sortait.
      Mais elle CLIQUE, et le banc ne sait pas le faire — les captures
      non plus, les clics simulés n'atteignant pas l'application sous
      Wayland. Or tous nos contrôles d'interface fabriquent l'état
      (`flow_pseudo: true`) puis mesurent ce qui est peint : ils
      vérifient que la vue est juste, jamais que le geste y mène.
      Débrancher la case « Pseudo-code » ne faisait rougir aucun test ;
      elle en fait rougir un maintenant.
      L'emploi est borné : kittest pour les GESTES, le banc pour la mise
      en page — il mesure des pixels, là où l'arbre d'accessibilité ne
      connaît que des noms.

## 7. Dette technique

- [~] **Le découpage de `main.rs`** (A 5.1, 7.2) — 20 641 lignes au
      départ, 6 440 aujourd'hui. Dix-neuf modules en sont sortis :
      `flow`, `tools`, `windows`, `editor`, `menus`, `calc_ui`,
      `console`, `help`, `preferences`, `statusbar`, `run`, `files`,
      `jobs`, `marks`, `editing`, `maximize`, `session`, `exercises`,
      `text`. Ce qui reste est la structure `NamsdecApp`, ses
      énumérations, le routeur d'actions, les fonctions de dessin
      partagées et la boucle de rendu — c'est-à-dire ce qui a le plus
      de raisons d'y rester.
      Le patron est celui de `sidebar.rs` : `impl NamsdecApp` dans le
      module, `use super::*` en tête, le texte déplacé sans une virgule
      de différence, et `pub(crate)` seulement sur ce qu'un module frère
      appelle.
- [x] **Aucun commentaire n'a perdu sa fonction** — vingt-cinq blocs de
      `///` documentaient autre chose que ce qu'ils précédaient, dont
      vingt-deux avant le découpage. Rust laisse deux blocs se suivre et
      les concatène : rien ne le signalait. Six recollés, cinq effacés,
      un invariant pose la règle.
- [x] **Les contrôles qui lisent le code voient tous les modules**
      (A 7.11) — quatre d'entre eux ne regardaient que `main.rs` et se
      vidaient à chaque extraction, sans rougir. `SOURCES_DE_L_INTERFACE`
      les branche sur les vingt-neuf fichiers de l'interface, et un
      contrôle vérifie que cette liste suit les `mod`.
- [~] **Le listing entier est mis en page en une fois** (A 5.3, 7.8) —
      MESURÉ le 30 septembre, et ce n'était pas où l'on croyait.
      Les trois cents millisecondes de la mise en page sont payées UNE
      fois : le cache tenait déjà. Ce qui se payait à chaque frame était
      la CLÉ de ce cache — un condensé des six cent mille octets, 1,2 ms,
      deux fois par frame. Le texte lui-même sert de clé maintenant :
      11 µs, cent treize fois moins, et plus sûr.
      Ce qui reste : les ~280 ms de la première pose, à l'ouverture d'un
      binaire. Les retirer demande de ne poser que les lignes visibles,
      donc de sortir du `TextEdit` d'egui — qui porte le curseur, la
      sélection, la recherche, les points d'arrêt et la complétion.
      C'est un vrai chantier, non un réglage.
- [x] **Les frames lentes** (A 5.2) — le message ne confond plus les
      deux causes. La distinction se devinait en comparant la longueur
      du texte d'une frame à l'autre, ce qui marche quand l'éditeur pose
      dans la même frame et se trompe exactement quand il ne faut pas :
      un fichier venu d'un dialogue arrive parfois après le dessin, et
      c'est la frame SUIVANTE qui paie. Le cache de mise en page compte
      ses poses — il sait, lui, s'il a payé.
- [x] **Ce qui pèse dans le binaire** (A 5.4) — remesuré le 30
      septembre, et tranché. La pile TLS de la recherche de mise à jour
      pèse 1,6 Mo sur 16, autant que tout le code de namsdec réuni, pour
      une fonction employée une fois par mois : elle passe derrière une
      option de compilation, activée par défaut. Un empaqueteur bâtit
      avec `--no-default-features` et obtient 14,4 Mo.
      `panic = "abort"` gagnerait autant et n'est PAS pris : une panique
      dans l'un des cinq threads de fond tuerait alors tout le processus
      au lieu du seul thread, et namsdec en lance pour installer,
      décompiler et chercher une version. Un mégaoctet et demi ne vaut
      pas le travail en cours de quelqu'un.
- [x] `egui_extras` retiré des dépendances (A 5.5)
- [x] **Dépendances à jour** (7 octobre 2026) — contrôle complet :
      toutes les dépendances directes étaient déjà sur leur dernière
      version majeure (egui/eframe 0.36.2, iced-x86 1.21, rfd 0.17.2,
      arboard 3.6.1, ureq 3.4.2, egui_code_editor 0.4.1) — aucun saut
      majeur à faire. `cargo update` a tiré les correctifs compatibles
      (thiserror 2.0.21, libc, rustix, zerocopy, wasm-bindgen, cc…).
      `tokio` apparaît dans le lock via `wasm-bindgen-futures` mais
      seulement sur la cible wasm32 : il n'entre pas dans le binaire
      natif. Compilation propre, clippy muet, 1 087 tests au vert.

---

## Comment cette liste se tient

Une entrée passe à `[x]` quand **un contrôle la retient** — pas quand le
code est écrit. C'est la règle qui a le plus servi cette année : trois
défauts corrigés sans invariant sont revenus, et un contrôle écrit sans
être éprouvé en remettant le défaut est passé au vert pendant des
semaines en ne regardant rien.

Toute modification visible met aussi à jour `docs/guide.py` et régénère
le PDF du manuel.
