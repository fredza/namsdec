/* Scénario fermé du mode simple : une boucle et une fonction.
 *
 * Aucun code utilisateur n'est exécuté, aucun assembleur ni émulateur
 * n'est appelé. Les 23 états sont dérivés une fois pour toutes au
 * chargement, par la petite machine ci-dessous.
 *
 * Le programme : la somme de 1 à 5, calculée dans une fonction appelée
 * par `call`. Deux instructions de plus que l'exemple du 29 octobre, et
 * pour une raison : sans `call`, la pile ne bouge jamais, et un endroit
 * où regarder qui ne bouge pas n'apprend rien.
 */
'use strict';
globalThis.Demo = (() => {
  const SOURCE = [
    { texte: 'section .text', genre: 'directive' },
    { texte: '    global _start', genre: 'directive' },
    { texte: '', genre: 'vide' },
    { texte: '_start:', genre: 'label' },
    { texte: '    mov ecx, 5', genre: 'instr', ip: 0, note: 'cinq tours à faire' },
    { texte: '    xor eax, eax', genre: 'instr', ip: 1, note: 'la somme part de zéro' },
    { texte: '    call somme', genre: 'instr', ip: 2 },
    { texte: '    mov edi, eax', genre: 'instr', ip: 3 },
    { texte: '    mov eax, 60', genre: 'instr', ip: 4 },
    { texte: '    syscall', genre: 'instr', ip: 5 },
    { texte: '', genre: 'vide' },
    { texte: 'somme:', genre: 'label' },
    { texte: '.boucle:', genre: 'label' },
    { texte: '    add rax, rcx', genre: 'instr', ip: 6 },
    { texte: '    dec rcx', genre: 'instr', ip: 7 },
    { texte: '    jnz .boucle', genre: 'instr', ip: 8 },
    { texte: '    ret', genre: 'instr', ip: 9 }
  ];

  /* Les dix instructions, dans l'ordre du listing. */
  const ops = [
    'mov ecx, 5', 'xor eax, eax', 'call somme', 'mov edi, eax',
    'mov eax, 60', 'syscall', 'add rax, rcx', 'dec rcx', 'jnz .boucle', 'ret'
  ];
  const addresses = [
    0x401000, 0x401005, 0x401007, 0x40100c,
    0x40100e, 0x401013, 0x401015, 0x401018, 0x40101b, 0x40101d
  ];
  const bytes = [
    'b9 05 00 00 00', '31 c0', 'e8 09 00 00 00', '89 c7',
    'b8 3c 00 00 00', '0f 05', '48 01 c8', '48 ff c9', '75 f8', 'c3'
  ];
  const FIN_DU_PROGRAMME = 0x40101e;
  const RSP_DEPART = 0x7ffddfffee00;
  const RETOUR = addresses[3];

  const states = [{
    rax: 0, rcx: 0, rdi: 0, zf: 0, rsp: RSP_DEPART,
    pile: [], pc: 0, done: false, pas: 0, faite: null
  }];
  while (!states.at(-1).done && states.length < 64) {
    const s = states.at(-1);
    const n = { ...s, pile: s.pile.slice(), pas: s.pas + 1, faite: s.pc };
    switch (s.pc) {
      case 0: n.rcx = 5; n.pc = 1; break;
      case 1: n.rax = 0; n.zf = 1; n.pc = 2; break;
      case 2:
        n.rsp = s.rsp - 8;
        n.pile = [{ adresse: n.rsp, valeur: RETOUR, quoi: 'retour' }, ...s.pile];
        n.pc = 6;
        break;
      case 3: n.rdi = s.rax; n.pc = 4; break;
      case 4: n.rax = 60; n.pc = 5; break;
      case 5: n.done = true; break;
      case 6: n.rax = s.rax + s.rcx; n.zf = Number(n.rax === 0); n.pc = 7; break;
      case 7: n.rcx = s.rcx - 1; n.zf = Number(n.rcx === 0); n.pc = 8; break;
      case 8: n.pc = s.zf ? 9 : 6; break;
      case 9:
        n.pile = s.pile.slice(1);
        n.rsp = s.rsp + 8;
        n.pc = 3;
        break;
    }
    states.push(n);
  }

  const hex = (n, large = 8) => n.toString(16).padStart(large, '0');
  const hex16 = n => n.toString(16).padStart(12, '0');
  const rip = s => (s.done ? FIN_DU_PROGRAMME : addresses[s.pc]);
  const ligne_de = ip => SOURCE.findIndex(l => l.ip === ip);

  /* Ce que vaut un registre, et s'il vient de changer. */
  const REGISTRES = [
    { nom: 'RAX', clef: 'rax', role: 'la somme' },
    { nom: 'RCX', clef: 'rcx', role: 'ce qui reste à compter' },
    { nom: 'RDI', clef: 'rdi', role: 'ce qu’on rendra au système' },
    { nom: 'RSP', clef: 'rsp', role: 'le haut de la pile' }
  ];

  /* Les fiches : ce que namsdec-doc dirait de ces dix lignes. */
  const fiches = {
    mov: ['mov', 'Copier', 'Recopie la valeur de droite dans l’emplacement de gauche. Rien n’est effacé à droite, et aucun indicateur ne change.'],
    xor: ['xor', 'Ou exclusif', 'Compare les bits deux à deux. Un registre contre lui-même donne toujours zéro : c’est la façon la plus courte d’écrire « remets à zéro ».'],
    call: ['call', 'Appeler', 'Écrit sur la pile l’adresse de la ligne suivante, puis saute. C’est ce papier posé sur la pile qui permettra à ret de retrouver son chemin.'],
    add: ['add', 'Ajouter', 'Ajoute la valeur de droite à celle de gauche, et met à jour les indicateurs selon le résultat.'],
    dec: ['dec', 'Diminuer de un', 'Retire 1. ZF passe à 1 si le résultat est nul : c’est ainsi qu’une boucle sait qu’elle a fini.'],
    jnz: ['jnz', 'Sauter si ce n’est pas zéro', 'Regarde ZF. S’il vaut 0, l’exécution reprend à l’étiquette ; s’il vaut 1, elle continue à la ligne d’en dessous.'],
    ret: ['ret', 'Revenir', 'Reprend sur la pile l’adresse posée par call, et y retourne. La pile redescend d’autant.'],
    syscall: ['syscall', 'Demander au système', 'Passe la main au noyau. Le numéro du service est dans RAX, ses arguments dans RDI, RSI, RDX. Ici RAX vaut 60 : terminer.']
  };
  const fiche_de = ip => fiches[ops[ip].split(/[ ,]/)[0]];

  /* Le titre, la phrase et le calcul de l'état courant. Trois niveaux
   * de détail, pour trois maquettes qui n'ont pas la même place. */
  function explain(s) {
    if (s.done) {
      return ['Le programme est terminé',
        `Il rend ${s.rdi} au système. Ce nombre est ce que RDI contenait au moment du syscall — c’est la somme calculée par la fonction.`,
        `exit(${s.rdi})`];
    }
    const t = [
      ['Donner cinq tours à faire', 'RCX reçoit 5. C’est le compteur de la boucle : il descendra jusqu’à zéro.', 'RCX ← 5'],
      ['Partir de zéro', 'Un registre contre lui-même en ou exclusif donne zéro. RAX est prêt à accueillir la somme.', 'RAX ← 0'],
      ['Appeler la fonction', 'L’adresse de la ligne suivante est posée sur la pile, puis l’exécution saute à somme.', `pile ← 0x${hex(RETOUR, 6)}`],
      ['Garder le résultat pour la sortie', 'RDI est le premier argument d’un appel système. La somme y passe.', `RDI ← ${s.rax}`],
      ['Choisir le service', '60 est le numéro de exit sous Linux x86-64.', 'RAX ← 60'],
      ['Terminer', 'Le noyau prend la main et arrête le programme avec le code reçu dans RDI.', `exit(${s.rdi})`],
      ['Ajouter le compteur à la somme', 'RAX grandit de la valeur de RCX. RCX, lui, ne bouge pas.', `${s.rax} + ${s.rcx} = ${s.rax + s.rcx}`],
      ['Décompter un tour', 'RCX perd 1. ZF dira si l’on vient d’atteindre zéro.', `${s.rcx} − 1 = ${s.rcx - 1}`],
      [s.zf ? 'Sortir de la boucle' : 'Refaire un tour',
        s.zf ? 'ZF vaut 1 : le compteur est à zéro, le saut n’est pas pris, on continue sur ret.'
             : 'ZF vaut 0 : il reste à compter, l’exécution repart à .boucle.',
        `ZF = ${s.zf} → ${s.zf ? 'on continue' : 'on repart'}`],
      ['Revenir d’où l’on venait', 'ret reprend sur la pile l’adresse posée par call. La pile redescend de huit octets.', `retour en 0x${hex(RETOUR, 6)}`]
    ][s.pc];
    return t;
  }

  /* Ce qui a changé entre deux états : de quoi souligner une valeur
   * sans que chaque maquette ait à le recalculer. */
  function diff(avant, apres) {
    if (!avant) return new Set();
    const bouge = new Set();
    for (const clef of ['rax', 'rcx', 'rdi', 'rsp', 'zf']) {
      if (avant[clef] !== apres[clef]) bouge.add(clef);
    }
    if (avant.pile.length !== apres.pile.length) bouge.add('pile');
    return bouge;
  }

  /* L'erreur que nasm rendrait sur la faute la plus commune du premier
   * jour : un registre 32 bits mêlé à un registre 64 bits. Les trois
   * maquettes la montrent sur demande — un écran se juge aussi à ce
   * qu'il fait quand ça rate. */
  const erreur = {
    ligne: 14,
    colonne: 5,
    texte: 'somme.asm:14: error: invalid combination of opcode and operands',
    source: '    add rax, ecx',
    titre: 'Deux tailles mêlées sur la même ligne',
    explication: 'RAX fait 64 bits, ECX en fait 32. Une addition veut deux valeurs de même taille.',
    correction: 'add rax, rcx'
  };

  return {
    SOURCE, ops, addresses, bytes, states, REGISTRES, fiches,
    hex, hex16, rip, ligne_de, explain, diff, fiche_de, erreur,
    RSP_DEPART, FIN_DU_PROGRAMME
  };
})();
