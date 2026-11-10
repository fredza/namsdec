/* Ce que les trois maquettes partagent : colorer une ligne, dire une
 * valeur, et tenir le temps. Rien d'autre — leur mise en page n'a
 * justement pas à se ressembler. */
'use strict';
globalThis.Commun = (() => {
  const MNEMONIQUES = new Set([
    'mov', 'xor', 'call', 'add', 'dec', 'jnz', 'ret', 'syscall', 'push', 'pop', 'cmp', 'jmp'
  ]);
  const REGISTRES = /^(r[a-d]x|r[sd]i|r[sb]p|r8|r9|r1[0-5]|e[a-d]x|e[sd]i|e[sb]p|[a-d][lhx])$/i;

  /* Colorer une ligne de source. Pas un analyseur : un découpage qui
   * suffit à dix lignes connues d'avance, et qui échappe tout ce qu'il
   * pose dans le document. */
  function colorer(texte) {
    const indent = texte.match(/^\s*/)[0];
    let reste = texte.slice(indent.length);
    let commentaire = '';
    const point = reste.indexOf(';');
    if (point >= 0) {
      commentaire = reste.slice(point);
      reste = reste.slice(0, point);
    }
    const morceaux = reste.split(/(\s+|,)/).map(mot => {
      if (!mot.trim()) return esc(mot);
      const nu = mot.trim().toLowerCase();
      if (nu === ',') return ',';
      if (MNEMONIQUES.has(nu)) return `<span class="mn">${esc(mot)}</span>`;
      if (REGISTRES.test(nu)) return `<span class="rg">${esc(mot)}</span>`;
      if (/^-?(0x[0-9a-f]+|\d+)$/.test(nu)) return `<span class="nb">${esc(mot)}</span>`;
      if (nu.startsWith('section') || nu.startsWith('global') || nu.startsWith('db')) {
        return `<span class="dr">${esc(mot)}</span>`;
      }
      if (mot.endsWith(':') || nu.startsWith('.') || nu === 'somme' || nu === '_start') {
        return `<span class="lb">${esc(mot)}</span>`;
      }
      return esc(mot);
    }).join('');
    const fin = commentaire ? `<span class="cm">${esc(commentaire)}</span>` : '';
    return esc(indent) + morceaux + fin;
  }

  const esc = t => t.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

  /* Une valeur de registre, dite deux fois : en décimal pour qui
   * apprend, en hexadécimal parce que c'est ce que montre un
   * débogueur. L'ordre n'est pas neutre — le décimal d'abord. */
  function valeur(n, large) {
    return { dec: String(n), hex: '0x' + n.toString(16).padStart(large || 2, '0') };
  }
  const adresse = n => '0x' + n.toString(16);

  /* Le temps. Un seul objet le tient, les vues s'y abonnent : deux
   * compteurs auraient fini par se contredire, et c'est exactement le
   * défaut qu'on cherche à éviter dans l'application. */
  function Lecture(nb_etats, surChangement) {
    let i = 0, auto = null, erreur = false;
    const dire = () => surChangement({
      i, auto: auto !== null, erreur,
      debut: i === 0, fin: i === nb_etats - 1
    });
    const api = {
      get index() { return i; },
      get en_erreur() { return erreur; },
      get en_marche() { return auto !== null; },
      aller(n) {
        const borne = Math.max(0, Math.min(nb_etats - 1, n));
        if (borne !== i) { i = borne; dire(); }
        return api;
      },
      pas() { api.stop(); return api.aller(i + 1); },
      recul() { api.stop(); return api.aller(i - 1); },
      reprendre() { api.stop(); erreur = false; return api.aller(0); },
      /* L'exécution automatique s'arrête d'elle-même à la fin, et
       * quand l'onglet passe en arrière-plan : une maquette qui
       * continue de tourner sans personne devant trompe le lecteur sur
       * ce qu'elle a montré. */
      marche(cadence = 420) {
        if (auto !== null || i === nb_etats - 1) return api;
        auto = setInterval(() => {
          if (i >= nb_etats - 1) { api.stop(); return; }
          i += 1; dire();
        }, cadence);
        dire();
        return api;
      },
      stop() {
        if (auto !== null) { clearInterval(auto); auto = null; dire(); }
        return api;
      },
      bascule(cadence) { return auto !== null ? api.stop() : api.marche(cadence); },
      echouer(oui) { erreur = oui; dire(); return api; },
      rafraichir() { dire(); return api; }
    };
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) api.stop();
    });
    return api;
  }

  /* Les raccourcis que les trois partagent. Jamais capturés quand on
   * est en train de taper quelque part. */
  function clavier(lecture, extra) {
    document.addEventListener('keydown', e => {
      const cible = e.target;
      if (cible && (cible.isContentEditable || /^(input|textarea|select)$/i.test(cible.tagName))) return;
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      const touche = e.key;
      if (touche === 'F10' && e.shiftKey) { lecture.recul(); }
      else if (touche === 'F10') { lecture.pas(); }
      else if (touche === 'ArrowRight') { lecture.pas(); }
      else if (touche === 'ArrowLeft') { lecture.recul(); }
      else if (touche === ' ' || touche === 'F5') { lecture.bascule(); }
      else if (touche === 'Home') { lecture.reprendre(); }
      else if (extra && extra(touche, e)) { /* la vue a pris la touche */ }
      else return;
      e.preventDefault();
    });
  }

  /* D'où l'on part : le pas 4 par défaut — la boucle a déjà tourné,
   * donc la pile porte quelque chose et une valeur vient de changer,
   * et un écran vide ne se juge pas. `?pas=N` et `?erreur=1` ouvrent
   * n'importe quel état : c'est ce qui rend une capture reproductible
   * et un contrôle écrivable. */
  function depart(lecture, defaut = 4) {
    const p = new URLSearchParams(location.search);
    const n = Number(p.get('pas'));
    lecture.aller(Number.isFinite(n) && p.has('pas') ? n : defaut);
    if (p.get('erreur') === '1') lecture.echouer(true);
    return lecture;
  }

  return { colorer, esc, valeur, adresse, Lecture, clavier, depart };
})();
