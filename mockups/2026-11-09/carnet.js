/* Le Carnet : la page, la note en marge, et la prévision.
 *
 * La règle de la prévision est celle déjà retenue dans l'application
 * (prevision.rs) : on répond AVANT, le verdict n'arrive qu'APRÈS le
 * pas, et il ne donne jamais la bonne réponse — on l'obtient en
 * exécutant, ce que l'exercice enseigne.
 */
'use strict';
(() => {
  const D = globalThis.Demo, C = globalThis.Commun;
  const $ = id => document.getElementById(id);

  /* Ce qu'on a prédit, pour le pas qu'on s'apprête à faire. */
  let prediction = null;      // { depuis, valeur, libelle }
  let a_juger = null;         // la prédiction du pas qu'on vient de faire

  /* Les questions. Une par instruction qui en mérite une — les deux
   * `mov` de préparation n'en méritent pas : il n'y a rien à deviner. */
  /* La bonne réponse est écrite en premier dans chaque question, parce
   * que c'est ainsi qu'on les relit. Elle ne doit surtout pas être
   * PROPOSÉE en premier : on apprendrait à cliquer à gauche, pas à
   * lire le programme. Une rotation qui dépend du pas la déplace —
   * déterministe, donc une capture reste comparable, et un contrôle
   * sait d'avance où est la réponse. */
  function tourner(options, n) {
    const k = n % options.length;
    return options.slice(k).concat(options.slice(0, k));
  }

  function question(s) {
    const q = (texte, options, lire) => ({ texte, options: tourner(options, s.pas), lire });
    switch (s.pc) {
      case 1: return q('Après cette ligne, que vaudra RAX ?',
        [['0', 0], [String(s.rcx), s.rcx], ['on ne peut pas savoir', null]], e => e.rax);
      case 2: return q('Sur quelle ligne l’exécution va-t-elle continuer ?',
        [['ligne 14 — add rax, rcx', 6], ['ligne 8 — mov edi, eax', 3], ['ligne 17 — ret', 9]], e => e.pc);
      case 3: return q('Que vaudra RDI ?',
        [[String(s.rax), s.rax], ['0', 0], ['60', 60]], e => e.rdi);
      case 5: return q('Que va-t-il se passer ?',
        [['le programme s’arrête', 1], ['la boucle repart', 2], ['RAX est affiché', 3]],
        e => (e.done ? 1 : 2));
      case 6: return q('Après cette addition, que vaudra RAX ?',
        [[String(s.rax + s.rcx), s.rax + s.rcx], [String(s.rcx), s.rcx], [String(s.rax), s.rax]], e => e.rax);
      case 7: return q('Après cette ligne, que vaudra RCX ?',
        [[String(s.rcx - 1), s.rcx - 1], [String(s.rcx), s.rcx], ['0', 0]], e => e.rcx);
      case 8: return q('Le saut sera-t-il pris ?',
        [['oui, on repart à .boucle', 6], ['non, on continue sur ret', 9]], e => e.pc);
      case 9: return q('Où revient-on ?',
        [['ligne 8 — mov edi, eax', 3], ['ligne 5 — mov ecx, 5', 0], ['ligne 14 — add rax, rcx', 6]], e => e.pc);
      default: return null;
    }
  }

  /* Les deux lignes d'en-tête : repliées d'office, dépliables d'un
   * clic. On les écrit une fois et on ne les relit jamais, et deux
   * lignes de moins, c'est la question de prévision qui remonte
   * au-dessus du pli. */
  const TETE = [0, 1, 2];
  let tete_repliee = true;
  const bouton_tete = document.createElement('button');
  bouton_tete.className = 'entete-repliee';
  bouton_tete.addEventListener('click', () => {
    tete_repliee = !tete_repliee;
    rendre_tete();
  });
  $('programme').appendChild(bouton_tete);

  /* Le programme, posé une fois. */
  const rangees = D.SOURCE.map((l, n) => {
    const r = document.createElement('div');
    r.className = 'rangee';
    r.innerHTML =
      `<div class="source"><span class="no">${n + 1}</span><span class="txt">${C.colorer(l.texte) || '&nbsp;'}</span></div>` +
      `<div class="marge"></div>`;
    $('programme').appendChild(r);
    return r;
  });

  function rendre_tete() {
    bouton_tete.textContent = tete_repliee
      ? '▸ l’en-tête du fichier (2 lignes)'
      : '▾ replier l’en-tête du fichier';
    TETE.forEach(n => rangees[n].classList.toggle('cachee', tete_repliee));
  }
  rendre_tete();

  const lecture = C.Lecture(D.states.length, () => peindre());

  function peindre() {
    const i = lecture.index;
    const s = D.states[i];
    const avant = i > 0 ? D.states[i - 1] : null;
    const bouge = D.diff(avant, s);
    const casse = lecture.en_erreur;
    /* À la fin, la ligne marquée est celle qui a terminé le programme
     * — `syscall`. Ne marquer aucune ligne laisse la page sans point
     * d'arrivée, et on cherche où l'on en est. */
    const ici = casse ? D.erreur.ligne - 1 : D.ligne_de(s.done ? 5 : s.pc);

    rangees.forEach((r, n) => {
      const src = D.SOURCE[n];
      r.classList.toggle('ici', !casse && n === ici);
      r.classList.toggle('finie', !casse && s.done && n === ici);
      r.classList.toggle('fautive', casse && n === ici);
      r.querySelector('.txt').innerHTML =
        (casse && n === D.erreur.ligne - 1 ? C.colorer(D.erreur.source) : C.colorer(src.texte)) || '&nbsp;';
      r.querySelector('.marge').innerHTML = '';
    });

    /* La note, en face de la ligne courante. Elle dit ce que la ligne
     * VA faire — c'est là qu'on est, pas là qu'on était. */
    if (casse) {
      poser_note(ici, `<div class="note erreur"><b>${D.erreur.titre}</b>${D.erreur.explication}</div>`);
    } else if (s.done) {
      poser_note(ici, `<div class="note fin"><b>Le programme s’arrête ici</b>
        Le noyau a pris la main et rendu <b style="display:inline">${s.rdi}</b> — la somme, passée par RDI.
        <span class="calcul">exit(${s.rdi})</span>
        <span class="octets">22 instructions exécutées pour 10 lignes écrites</span></div>`);
    } else if (ici >= 0) {
      const [titre, texte, calcul] = D.explain(s);
      poser_note(ici, `<div class="note"><b>${titre}</b>${texte}
        <span class="calcul">${C.esc(calcul)}</span>
        <span class="octets">${D.bytes[s.pc]} — ${D.bytes[s.pc].split(' ').length} octets en 0x${D.rip(s).toString(16)}</span></div>`);
    }

    /* La prévision : la question du pas À VENIR, le verdict du pas
     * qu'on vient de faire. */
    peindre_prevision(s, casse);

    /* L'état, en une ligne de jetons. */
    $('machine').innerHTML = [
      jeton('RAX', s.rax, avant && avant.rax, bouge.has('rax') && !casse),
      jeton('RCX', s.rcx, avant && avant.rcx, bouge.has('rcx') && !casse),
      jeton('RDI', s.rdi, avant && avant.rdi, bouge.has('rdi') && !casse),
      `<span class="jeton drapeau${s.zf ? ' vrai' : ''}"><span class="nom">ZF</span><span class="val">${s.zf}</span></span>`
    ].join('');
    $('phrase-pile').innerHTML = s.pile.length
      ? `La pile porte une case : l’adresse <code>${C.adresse(s.pile[0].valeur)}</code>, posée par <code>call</code>. C’est là que <code>ret</code> ramènera l’exécution.`
      : `La pile est vide : rien n’y a encore été posé, ou <code>ret</code> vient de tout reprendre.`;

    /* Le résultat. */
    if (casse) {
      $('sortie').innerHTML =
        `<div class="faute">${C.esc(D.erreur.texte)}</div>
         <p>Rien n’a tourné. La ligne fautive est marquée dans le programme, et la note en marge dit pourquoi.</p>
         <button class="corriger" id="b-corriger">Corriger en « ${D.erreur.correction} »</button>`;
      $('b-corriger').addEventListener('click', () => lecture.echouer(false));
    } else if (s.done) {
      $('sortie').innerHTML =
        `<p><span class="fini">Terminé.</span> Le programme rend <span class="gros">${s.rdi}</span> au système — c’est la somme 5 + 4 + 3 + 2 + 1.</p>
         <p>Il n’a rien affiché : ce programme calcule, il ne parle pas. Pour voir le 15 depuis un terminal, on demanderait <span class="gros">echo $?</span> après l’avoir lancé.</p>`;
    } else if (i === 0) {
      $('sortie').innerHTML = `<p>Assemblé sans erreur : 10 instructions, 32 octets. Le programme n’a pas encore tourné.</p>`;
    } else {
      $('sortie').innerHTML = `<p>Le programme est en cours — ${s.pas} ligne${s.pas > 1 ? 's' : ''} exécutée${s.pas > 1 ? 's' : ''} sur 22. Rien n’a été affiché.</p>`;
    }

    $('ou').textContent = `${s.pas} / ${D.states.length - 1}`;
    $('b-pas').disabled = casse || s.done;
    $('b-recul').disabled = casse || i === 0;
    $('b-lancer').disabled = casse || s.done;
    $('b-lancer').textContent = lecture.en_marche ? '⏸ Arrêter' : '▶ Tout dérouler';
    $('b-reprendre').disabled = !casse && i === 0;
  }

  function poser_note(n, html) {
    if (n >= 0 && rangees[n]) rangees[n].querySelector('.marge').innerHTML = html;
  }

  function jeton(nom, v, av, bouge) {
    const ancien = bouge && av !== null && av !== undefined ? `<span class="avant">${av}</span>` : '';
    return `<span class="jeton${bouge ? ' bouge' : ''}"><span class="nom">${nom}</span>${ancien}<span class="val">${v}</span></span>`;
  }

  function peindre_prevision(s, casse) {
    const boite = $('prevision');
    const v = $('verdict');
    if (casse) { boite.hidden = true; return; }
    boite.hidden = false;

    /* Le verdict porte sur le pas qu'on vient de faire, et sur lui
     * seul : passé ce pas, il disparaît. Un verdict qui traîne finit
     * par être lu comme celui de la ligne courante. */
    if (a_juger && a_juger.apres === lecture.index) {
      const juste = a_juger.valeur !== null && a_juger.valeur === a_juger.lire(s);
      v.hidden = false;
      v.className = 'verdict ' + (juste ? 'ok' : 'non');
      v.textContent = juste
        ? `Vous aviez dit « ${a_juger.libelle} ». C’est ce que la machine a fait.`
        : `Vous aviez dit « ${a_juger.libelle} ». La machine a fait autre chose — la marge et les valeurs ci-dessous disent quoi.`;
    } else {
      a_juger = null;
      v.hidden = true;
    }

    const q = s.done ? null : question(s);
    if (!q) {
      $('question').textContent = s.done
        ? 'Le programme est terminé. Il n’y a plus rien à prévoir.'
        : 'Rien à deviner ici : cette ligne pose une valeur, et c’est tout.';
      $('choix').innerHTML = '';
      $('passer').hidden = true;
      return;
    }
    $('passer').hidden = false;
    $('question').textContent = q.texte;
    $('choix').innerHTML = q.options.map(([libelle], k) =>
      `<button data-k="${k}"${prediction && prediction.k === k ? ' class="choisi"' : ''}>${C.esc(libelle)}</button>`).join('');
    Array.from($('choix').children).forEach(b => {
      b.addEventListener('click', () => {
        const k = Number(b.dataset.k);
        const [libelle, valeur] = q.options[k];
        prediction = { k, libelle, valeur, lire: q.lire, apres: lecture.index + 1 };
        /* Marqué comme CHOISI, jamais comme juste : rien ici ne doit
         * laisser deviner la réponse avant le pas. */
        Array.from($('choix').children).forEach(x => x.classList.remove('choisi'));
        b.classList.add('choisi');
        v.hidden = false;
        v.className = 'verdict';
        v.textContent = `Prévision retenue : « ${libelle} ». Avancez d’un pas pour voir.`;
      });
    });
  }

  function avancer() {
    a_juger = prediction;
    prediction = null;
    lecture.pas();
  }

  $('b-pas').addEventListener('click', avancer);
  $('b-recul').addEventListener('click', () => { prediction = null; a_juger = null; lecture.recul(); });
  $('b-lancer').addEventListener('click', () => { prediction = null; a_juger = null; lecture.bascule(620); });
  $('b-reprendre').addEventListener('click', () => { prediction = null; a_juger = null; lecture.reprendre(); });
  $('passer').addEventListener('click', avancer);
  $('b-erreur').addEventListener('click', () => {
    prediction = null; a_juger = null;
    lecture.aller(0);
    lecture.echouer(!lecture.en_erreur);
  });

  C.clavier({
    pas: avancer,
    recul: () => { prediction = null; a_juger = null; lecture.recul(); },
    bascule: () => { prediction = null; a_juger = null; lecture.bascule(620); },
    reprendre: () => { prediction = null; a_juger = null; lecture.reprendre(); }
  });

  C.depart(lecture);
  peindre();
})();
