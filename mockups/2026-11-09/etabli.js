/* L'Établi : peint les quatre bandes à chaque changement d'état. */
'use strict';
(() => {
  const D = globalThis.Demo, C = globalThis.Commun;
  const $ = id => document.getElementById(id);
  const arrets = new Set();
  let fiche_ouverte = false;

  /* Le listing, posé une fois : seules les classes changent ensuite. */
  const lignes = D.SOURCE.map((l, n) => {
    const el = document.createElement('div');
    el.className = 'ligne';
    el.dataset.ip = l.ip ?? '';
    el.innerHTML =
      `<span class="arret" title="${l.genre === 'instr' ? 'Poser un point d’arrêt' : ''}">${l.genre === 'instr' ? '●' : ''}</span>` +
      `<span class="no">${n + 1}</span>` +
      `<span class="txt">${C.colorer(l.texte) || '&nbsp;'}</span>` +
      `<span class="octets">${l.ip !== undefined ? D.bytes[l.ip] : ''}</span>`;
    if (l.genre === 'instr') {
      el.querySelector('.arret').addEventListener('click', () => {
        arrets.has(n) ? arrets.delete(n) : arrets.add(n);
        peindre(lecture.index);
      });
    }
    $('lignes').appendChild(el);
    return el;
  });

  const lecture = C.Lecture(D.states.length, () => peindre(lecture.index));

  function peindre(i) {
    const s = D.states[i];
    const avant = i > 0 ? D.states[i - 1] : null;
    const bouge = D.diff(avant, s);
    const casse = lecture.en_erreur;
    const ligne_ici = casse ? D.erreur.ligne - 1 : (s.done ? -1 : D.ligne_de(s.pc));

    /* Les lignes. */
    lignes.forEach((el, n) => {
      el.classList.toggle('ici', !casse && n === ligne_ici);
      el.classList.toggle('fautive', casse && n === ligne_ici);
      const a = el.querySelector('.arret');
      if (a.textContent) a.classList.toggle('pose', arrets.has(n));
      const t = el.querySelector('.txt');
      const src = D.SOURCE[n];
      const fautive = casse && n === D.erreur.ligne - 1;
      t.innerHTML = (fautive ? C.colorer(D.erreur.source) : C.colorer(src.texte)) || '&nbsp;';
      /* Une ligne qui ne s'assemble pas n'a pas d'octets : les laisser
       * là ferait croire qu'elle a été traduite. */
      el.querySelector('.octets').textContent =
        fautive ? '—' : (src.ip !== undefined ? D.bytes[src.ip] : '');
    });
    if (ligne_ici >= 0) {
      lignes[ligne_ici].scrollIntoView({ block: 'nearest' });
    }

    /* La frise. */
    const etat = $('etat');
    etat.className = 'etat';
    if (casse) { etat.textContent = 'L’assemblage a échoué'; etat.classList.add('cassee'); }
    else if (s.done) { etat.textContent = 'Terminé'; etat.classList.add('fini'); }
    else if (lecture.en_marche) { etat.textContent = 'En marche'; etat.classList.add('marche'); }
    else if (i === 0) { etat.textContent = 'Prêt'; }
    else { etat.textContent = 'En pause'; }
    $('compte').textContent = casse ? 'rien n’a tourné' : `pas ${s.pas} / ${D.states.length - 1}`;
    $('jauge').style.width = casse ? '0%' : `${(i / (D.states.length - 1)) * 100}%`;

    /* La machine. */
    $('regs').innerHTML = D.REGISTRES.map(r => {
      const v = s[r.clef];
      const large = r.clef === 'rsp' ? 12 : 2;
      const hx = '0x' + v.toString(16).padStart(large, '0');
      const dec = r.clef === 'rsp' ? '<span class="val"></span>' : `<span class="val">${v}</span>`;
      return `<div class="reg${bouge.has(r.clef) && !casse ? ' bouge' : ''}">
        <span class="nom">${r.nom}</span>
        ${dec}
        <span class="hx">${hx}</span>
        <span class="role">${r.role}</span>
      </div>`;
    }).join('');

    const zf = $('zf');
    zf.textContent = `ZF ${s.zf}`;
    zf.classList.toggle('vrai', s.zf === 1);
    $('zf-dit').textContent = s.zf
      ? 'le dernier calcul a donné zéro'
      : 'le dernier calcul n’a pas donné zéro';

    $('pile').innerHTML = s.pile.length
      ? s.pile.map(c => `<div class="case-pile">
          <span class="ad">${C.adresse(c.adresse)}</span>
          <span class="vl">${C.adresse(c.valeur)}</span>
          <span class="quoi">l’adresse où <code>ret</code> devra revenir</span>
        </div>`).join('')
      : `<div class="pile-vide">Vide pour l’instant. <code>call</code> y posera l’adresse de retour, <code>ret</code> la reprendra.</div>`;

    /* La ligne où l'on est. */
    if (casse) {
      $('ex-titre').textContent = D.erreur.titre;
      $('ex-texte').textContent = D.erreur.explication;
      $('ex-calcul').textContent = 'rien n’a été exécuté';
      $('b-fiche').hidden = true;
      fermer_fiche();
    } else {
      const [titre, texte, calcul] = D.explain(s);
      $('ex-titre').textContent = titre;
      $('ex-texte').textContent = texte;
      $('ex-calcul').textContent = calcul;
      const f = s.done ? null : D.fiche_de(s.pc);
      $('b-fiche').hidden = !f;
      if (f) {
        $('b-fiche').textContent = `Qu’est-ce que « ${f[0]} » ? ▸`;
        if (fiche_ouverte) remplir_fiche(f);
      }
    }

    /* Le résultat. */
    const sortie = $('sortie');
    if (casse) {
      sortie.innerHTML =
        `<div class="faute">${C.esc(D.erreur.texte)}</div>` +
        `<div class="aide">La ligne est surlignée en rouge dans le code. Rien n’a tourné : il n’y a pas de programme à lancer tant que cette ligne n’est pas corrigée.</div>` +
        `<button class="corriger" id="b-corriger">Corriger en « ${D.erreur.correction} »</button>`;
      $('b-corriger').addEventListener('click', () => lecture.echouer(false));
    } else if (s.done) {
      sortie.innerHTML =
        `<div class="fini">Le programme s’est terminé normalement. Code de retour : ${s.rdi}</div>` +
        `<div class="aide">Rien n’a été écrit à l’écran : ce programme calcule, il n’affiche pas. Le 15 est la somme, rendue au système par RDI.</div>`;
    } else if (i === 0) {
      sortie.innerHTML = `<div class="attente">Assemblé sans erreur — 10 instructions, 32 octets. Le programme n’a pas encore tourné.</div>`;
    } else {
      sortie.innerHTML = `<div class="attente">Le programme tourne. Rien n’a encore été écrit à l’écran.</div>`;
    }

    /* Les boutons. */
    $('b-lancer').disabled = casse || s.done;
    $('b-lancer').firstElementChild.textContent = lecture.en_marche ? '⏸' : '▶';
    $('b-lancer').lastElementChild.textContent = lecture.en_marche ? 'Pause' : 'Lancer';
    $('b-pas').disabled = casse || s.done;
    $('b-recul').disabled = casse || i === 0;
    $('b-reprendre').disabled = !casse && i === 0;
    $('modifie').hidden = !casse;
  }

  function remplir_fiche(f) {
    $('fiche-mot').textContent = f[0];
    $('fiche-sens').textContent = f[1];
    $('fiche-txt').textContent = f[2];
    $('fiche').hidden = false;
    fiche_ouverte = true;
  }
  function fermer_fiche() { $('fiche').hidden = true; fiche_ouverte = false; }

  $('b-lancer').addEventListener('click', () => lecture.bascule());
  $('b-pas').addEventListener('click', () => lecture.pas());
  $('b-recul').addEventListener('click', () => lecture.recul());
  $('b-reprendre').addEventListener('click', () => lecture.reprendre());
  $('b-fiche').addEventListener('click', () => {
    const s = D.states[lecture.index];
    if (fiche_ouverte) fermer_fiche();
    else if (!s.done) remplir_fiche(D.fiche_de(s.pc));
  });
  $('fiche-fermer').addEventListener('click', fermer_fiche);
  $('b-erreur').addEventListener('click', () => {
    lecture.aller(0);
    lecture.echouer(!lecture.en_erreur);
  });

  C.clavier(lecture, touche => {
    if (touche === 'Escape' && fiche_ouverte) { fermer_fiche(); return true; }
    return false;
  });

  /* Ouvert au pas 4, comme les maquettes du 29 octobre : la boucle a
   * déjà tourné une fois, donc la pile porte quelque chose et un
   * registre vient de changer. Un écran vide ne se juge pas. */
  C.depart(lecture);
  peindre(lecture.index);
})();
