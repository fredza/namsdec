/* La Scène : deux temps, et un ruban qui se déplace sous la machine. */
'use strict';
(() => {
  const D = globalThis.Demo, C = globalThis.Commun;
  const $ = id => document.getElementById(id);

  /* ── Temps 1 : écrire ──────────────────────────────────────────── */
  const texte_juste = D.SOURCE.map(l => l.texte).join('\n');
  const texte_faux = D.SOURCE.map((l, n) => (n === D.erreur.ligne - 1 ? D.erreur.source : l.texte)).join('\n');
  $('texte').value = texte_juste;
  $('numeros').innerHTML = D.SOURCE.map((_, n) => n + 1).join('<br>');

  function temps(lequel) {
    const regarde = lequel === 'regarder';
    $('regarder').classList.toggle('vue', regarde);
    $('ecrire').classList.toggle('vue', !regarde);
    $('t-regarder').setAttribute('aria-selected', String(regarde));
    $('t-ecrire').setAttribute('aria-selected', String(!regarde));
    /* Le pied ne commande que le second temps : affiché pendant qu'on
     * écrit, il proposerait d'avancer dans une exécution qui n'a pas
     * commencé. */
    document.querySelector('.pied').style.display = regarde ? '' : 'none';
    if (regarde) recentrer();
  }

  /* ── Temps 2 : le ruban ────────────────────────────────────────── */
  const vignettes = D.ops.map((op, ip) => {
    const v = document.createElement('div');
    v.className = 'vignette';
    v.innerHTML = `<span class="ad">0x${D.addresses[ip].toString(16)}</span>${C.colorer(op)}`;
    $('bande').appendChild(v);
    return v;
  });

  function recentrer() {
    const s = D.states[lecture.index];
    const ip = s.done ? D.ops.length - 1 : s.pc;
    const v = vignettes[ip];
    const ruban = document.querySelector('.ruban');
    const dx = ruban.clientWidth / 2 - v.offsetLeft - v.offsetWidth / 2;
    $('bande').style.transform = `translateX(${dx}px)`;
  }
  window.addEventListener('resize', recentrer);

  const lecture = C.Lecture(D.states.length, () => peindre());

  function peindre() {
    const i = lecture.index;
    const s = D.states[i];
    const avant = i > 0 ? D.states[i - 1] : null;
    const bouge = D.diff(avant, s);
    const casse = lecture.en_erreur;

    if (casse) {
      /* Une faute d'assemblage ramène au premier temps : il n'y a rien
       * à regarder tourner, et c'est dans le texte que ça se corrige. */
      $('texte').value = texte_faux;
      $('verdict-asm').className = 'ko';
      $('verdict-asm').textContent = D.erreur.texte + '  —  ' + D.erreur.explication;
      $('b-regarder').textContent = `Corriger en « ${D.erreur.correction} »`;
      temps('ecrire');
      return;
    }
    $('texte').value = texte_juste;
    $('verdict-asm').className = 'ok';
    $('verdict-asm').textContent = 'Assemblé sans erreur — 10 instructions, 32 octets.';
    $('b-regarder').textContent = 'Regarder tourner ▸';

    const ip = s.done ? D.ops.length - 1 : s.pc;
    /* Ce qui a DÉJÀ tourné se distingue de ce qui n'a jamais tourné :
     * dans une boucle, c'est la seule façon de voir d'un coup d'œil
     * quelle part du programme on a traversée. */
    const vues = new Set(D.states.slice(1, i + 1).map(st => st.faite));
    vignettes.forEach((v, k) => {
      v.classList.toggle('ici', !s.done && k === ip);
      v.classList.toggle('vue', vues.has(k) && !(!s.done && k === ip));
      v.classList.toggle('jamais', !vues.has(k) && !(!s.done && k === ip));
    });
    recentrer();

    const [titre, dit, calcul] = D.explain(s);
    $('quand').textContent = s.done ? 'Fin du programme' : `Pas ${s.pas} sur ${D.states.length - 1}`;
    $('titre').textContent = titre;
    $('texte-dire').textContent = dit;

    /* Trois cartes seulement : celles que ce programme remue. Un
     * registre affiché qui ne bouge jamais apprend à ne plus regarder. */
    $('cartes').innerHTML = [
      ['RAX', 'rax', 'la somme'],
      ['RCX', 'rcx', 'ce qui reste à compter'],
      ['RDI', 'rdi', 'ce qu’on rendra au système']
    ].map(([nom, clef, role]) => {
      const v = s[clef];
      const change = bouge.has(clef);
      const depuis = change && avant ? `<b>${avant[clef]} → ${v}</b>` : `0x${v.toString(16)}`;
      return `<div class="carte${change ? ' bouge' : ''}">
        <div><div class="nom">${nom}</div><div class="role">${role}</div></div>
        <div>
          <div class="chiffre${String(v).length > 7 ? ' long' : ''}">${v}</div>
          <div class="depuis">${depuis}</div>
        </div>
      </div>`;
    }).join('');

    const f = s.done ? null : D.fiche_de(s.pc);
    if (f) {
      $('fiche').hidden = false;
      $('fiche-mot').textContent = f[0];
      $('fiche-sens').textContent = f[1];
      $('fiche-txt').textContent = f[2];
      $('fiche-octets').innerHTML =
        `cette ligne tient en ${D.bytes[s.pc].split(' ').length} octets : <b>${D.bytes[s.pc]}</b>, écrits en 0x${D.addresses[s.pc].toString(16)}`;
    } else {
      /* À la fin, la place de la fiche sert à lever le piège du
       * programme : RAX ne vaut pas 15 mais 60, et c'est RDI qui porte
       * la somme. Quelqu'un qui apprend lit la grande carte RAX en
       * premier et en conclut que sa boucle est fausse. */
      $('fiche').hidden = false;
      $('fiche-mot').textContent = 'RAX = 60 ?';
      $('fiche-sens').textContent = 'Non, la somme n’est pas perdue';
      $('fiche-txt').textContent =
        'La somme est dans RDI, où mov l’a mise. RAX a été réécrit juste après avec 60, '
        + 'le numéro de exit : c’est par lui qu’on dit au noyau quel service on demande. '
        + 'Les registres n’appartiennent à personne — ils servent à ce qu’on en fait, chacun son tour.';
      $('fiche-octets').innerHTML =
        'le programme a exécuté <b>22</b> instructions, pour <b>10</b> lignes écrites : la boucle en a repassé cinq fois.';
    }

    $('zf').classList.toggle('vrai', s.zf === 1);
    $('zf').querySelector('.code').textContent = `ZF ${s.zf}`;
    $('zf-dit').textContent = s.zf ? 'le dernier calcul a donné zéro' : 'le dernier calcul n’a pas donné zéro';
    $('rip').textContent = `0x${D.rip(s).toString(16)} — ${s.done ? 'après la dernière instruction' : 'la ligne qui va s’exécuter'}`;

    $('blocs').innerHTML = s.pile.map(c => `<div class="bloc">
        <div class="haut"><span>${C.adresse(c.valeur)}</span><span class="ad">${C.adresse(c.adresse)}</span></div>
        <div class="quoi">posée par <code>call</code> — c’est là que <code>ret</code> reviendra</div>
      </div>`).join('');
    $('rsp').innerHTML = `<b>RSP</b> 0x${s.rsp.toString(16)}${bouge.has('rsp') ? ` — a bougé de ${s.rsp > avant.rsp ? '+' : '−'}8 octets` : ''}`;

    $('sortie').innerHTML = s.done
      ? `<span class="fini">Terminé.</span> Le programme rend ${s.rdi} au système — la somme 5 + 4 + 3 + 2 + 1. Il n’a rien affiché : il calcule, il ne parle pas.`
      : `<span style="font-family:var(--mono)">${C.esc(calcul)}</span> — rien n’a encore été affiché à l’écran.`;

    $('compte').textContent = `${s.pas} / ${D.states.length - 1}`;
    $('jauge').style.width = `${(i / (D.states.length - 1)) * 100}%`;
    $('b-pas').disabled = s.done;
    $('b-lancer').disabled = s.done;
    $('b-lancer').textContent = lecture.en_marche ? '⏸ Arrêter' : '▶▶ Tout dérouler';
    $('b-recul').disabled = i === 0;
    $('b-reprendre').disabled = i === 0;
  }

  $('b-pas').addEventListener('click', () => lecture.pas());
  $('b-recul').addEventListener('click', () => lecture.recul());
  $('b-lancer').addEventListener('click', () => lecture.bascule(520));
  $('b-reprendre').addEventListener('click', () => lecture.reprendre());
  $('t-ecrire').addEventListener('click', () => temps('ecrire'));
  $('t-regarder').addEventListener('click', () => { if (!lecture.en_erreur) temps('regarder'); });
  $('b-regarder').addEventListener('click', () => {
    if (lecture.en_erreur) { lecture.echouer(false); temps('ecrire'); }
    else temps('regarder');
  });
  $('b-erreur').addEventListener('click', () => {
    lecture.aller(0);
    lecture.echouer(!lecture.en_erreur);
    if (!lecture.en_erreur) temps('regarder');
  });

  C.clavier(lecture);

  C.depart(lecture);
  peindre();
  temps('regarder');
})();
