/* Scénario fermé : somme de 1 à 5, sans exécution de code utilisateur. */
'use strict';
globalThis.Demo = (() => {
  const ops = ['mov ecx, 5', 'xor eax, eax', 'add rax, rcx', 'dec rcx', 'jnz boucle', 'mov edi, eax', 'mov eax, 60', 'syscall'];
  const addresses = [0x401000, 0x401005, 0x401007, 0x40100a, 0x40100d, 0x40100f, 0x401011, 0x401016];
  const bytes = ['b9 05 00 00 00', '31 c0', '48 01 c8', '48 ff c9', '75 f8', '89 c7', 'b8 3c 00 00 00', '0f 05'];
  const states = [{ rax: 0, rcx: 0, rdi: 0, zf: 0, pc: 0, done: false, last: 'Début de session' }];
  while (!states.at(-1).done) {
    const s = states.at(-1), n = { ...s, last: ops[s.pc] };
    switch (s.pc) {
      case 0: n.rcx = 5; n.pc = 1; break;
      case 1: n.rax = 0; n.zf = 1; n.pc = 2; break;
      case 2: n.rax += n.rcx; n.zf = Number(n.rax === 0); n.pc = 3; break;
      case 3: n.rcx--; n.zf = Number(n.rcx === 0); n.pc = 4; break;
      case 4: n.pc = s.zf ? 5 : 2; break;
      case 5: n.rdi = s.rax; n.pc = 6; break;
      case 6: n.rax = 60; n.pc = 7; break;
      case 7: n.done = true; break;
    }
    states.push(n);
  }
  const hex = n => n.toString(16).padStart(8, '0');
  const rip = s => s.done ? 0x401018 : addresses[s.pc];
  function explain(s) {
    if (s.done) return ['Programme terminé', `Le code de sortie vaut ${s.rdi}. Aucun texte n’a été écrit dans la console.`, `exit(${s.rdi})`];
    return [
      ['Initialiser le compteur', 'ECX reçoit 5. La moitié haute de RCX est remise à zéro.', 'RCX ← 5'],
      ['Partir de zéro', 'XOR d’un registre avec lui-même produit zéro. ZF passe à 1.', 'RAX ← 0'],
      ['Ajouter à la somme', 'Le compteur est ajouté à RAX. RCX conserve sa valeur.', `${s.rax} + ${s.rcx} = ${s.rax + s.rcx}`],
      ['Décompter un tour', 'RCX diminue de 1. ZF indiquera si le résultat est nul.', `${s.rcx} − 1 = ${s.rcx - 1}`],
      [s.zf ? 'Sortir de la boucle' : 'Revenir à l’addition', s.zf ? 'ZF vaut 1 : le saut ne sera pas pris.' : 'ZF vaut 0 : le saut sera pris vers boucle.', `ZF = ${s.zf} → ${s.zf ? 'suite' : 'boucle'}`],
      ['Conserver le résultat', 'RDI reçoit la somme, premier argument de l’appel système exit.', `RDI ← ${s.rax}`],
      ['Préparer la sortie', '60 est le numéro de exit sous Linux x86-64. La somme est dans RDI.', 'RAX ← 60'],
      ['Terminer le programme', 'exit reçoit le résultat par RDI. Il n’affiche pas de texte.', `exit(${s.rdi})`]
    ][s.pc];
  }
  return { ops, addresses, bytes, states, hex, rip, explain };
})();
