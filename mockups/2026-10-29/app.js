'use strict';
const { ops, addresses, bytes, states, hex, rip, explain } = Demo;
const design = document.body.dataset.design;
const names = { focus: 'Focus', carnet: 'Carnet', delta: 'Delta', prisme: 'Prisme' };
const root = document.getElementById('app');
let cursor = design === 'delta' ? 6 : 4, reference = 3, selected = 4;
let running = null, view = 'source', answer = null, showInspector = true, follow = true;
const button = (id, label, cls = '', disabled = false) => `<button id="${id}" data-action="${id}" class="${cls}" ${disabled ? 'disabled' : ''}>${label}</button>`;
const pill = text => `<span class="pill">${text}</span>`;
function controls(s) {
  return `<div class="controls">${button('back', '← Reculer', '', cursor === 0)}${button('step', 'Un pas → <kbd>F10</kbd>', 'primary', s.done)}${button('run', running ? 'Ⅱ Pause' : '▶ Exécuter', '', s.done)}${button('reset', '↺ Repartir')}</div>`;
}
function chrome(s) {
  return `<header class="chrome"><a class="brand" href="../index.html"><span class="brandmark">n.</span>namsdec</a><span class="designname">${names[design]}</span><nav aria-label="Autres maquettes">${Object.entries(names).map(([id, name]) => `<a href="${id}.html" ${id === design ? 'aria-current="page"' : ''}>${name}</a>`).join('')}</nav><a class="gallery-link" href="../index.html">Toutes les pistes ↗</a></header><div class="session"><span><span class="status-dot"></span> somme.asm <span class="muted">/ NASM x86-64</span></span>${pill(s.done ? 'Terminé · code 15' : running ? 'Exécution en cours' : 'Session en pause')}</div>`;
}
function tabs() {
  return `<div class="tabs" aria-label="Vue du code">${button('source', 'Source', view === 'source' ? 'active' : '')}${button('listing', 'Désassemblage', view === 'listing' ? 'active' : '')}<span class="muted">Lecture seule</span></div>`;
}
function code(s, compact = false) {
  const source = [
    ['; Somme des entiers de 1 à 5', 'comment'], ['section .text', 'directive'], ['global _start', 'directive'], ['', ''], ['_start:', 'label'],
    [ops[0], '', 0], [ops[1], '', 1], ['', ''], ['boucle:', 'label'], [ops[2], '', 2], [ops[3], '', 3], [ops[4], '', 4], ['', ''], [ops[5], '', 5], [ops[6], '', 6], [ops[7], '', 7]
  ];
  const rows = view === 'listing' || compact ? ops.map((op, i) => [op, '', i]) : source;
  return `<div class="code-scroll"><div class="code-lines ${compact ? 'compact' : ''}">${rows.map(([text, cls, pc], i) => {
    const active = pc === s.pc && !s.done;
    const textHtml = cls ? text : text.replace(/^(mov|xor|add|dec|jnz|syscall)/, '<b>$1</b>');
    return `<div class="code-line ${active ? 'current' : ''}" ${active ? 'aria-current="step"' : ''}><span class="gutter">${active ? '→' : i + 1}</span>${view === 'listing' || compact ? `<span class="addr">${hex(addresses[pc])}</span>` : ''}<code class="${cls}">${textHtml || '&nbsp;'}</code>${active ? '<span class="here">RIP</span>' : ''}</div>`;
  }).join('')}</div></div>`;
}
function registers(s) {
  const prev = states[Math.max(0, cursor - 1)];
  return `<div class="registers">${['rax', 'rcx', 'rdi'].map(reg => `<div class="register ${prev[reg] !== s[reg] ? 'changed' : ''}"><span>${reg.toUpperCase()}</span><code>${hex(s[reg])}</code><strong>${s[reg]}</strong></div>`).join('')}<div class="register"><span>RIP</span><code>${hex(rip(s))}</code><span></span></div></div><div class="flags"><span>ZF <strong>${s.zf}</strong></span><span>CF 0</span><span>SF 0</span><span>OF 0</span></div>`;
}
function explanation(s) {
  const [title, text, equation] = explain(s);
  return `<div class="explanation"><div class="eyebrow">${s.done ? 'Résultat' : 'À la prochaine instruction'}</div><h2>${title}</h2><code class="equation">${equation}</code><p>${text}</p></div>`;
}
function timeline() {
  return `<div class="timeline"><div><label for="time">État de la machine</label><span>Pas ${cursor} / ${states.length - 1}</span></div><input id="time" type="range" min="0" max="${states.length - 1}" value="${cursor}"><div class="muted"><span>Départ</span><span>Somme calculée</span><span>exit(15)</span></div></div>`;
}
function foot(s) {
  return `<footer><span>Démonstration interactive · scénario fixe · aucune exécution native</span><span>RIP ${hex(rip(s))} · pas ${cursor}</span></footer><p class="sr-only" role="status" aria-live="polite">Pas ${cursor}. RAX ${s.rax}, RCX ${s.rcx}, RDI ${s.rdi}. ${s.done ? 'Terminé.' : 'Prochaine instruction : ' + ops[s.pc]}</p>`;
}
function focus(s) {
  return `<main class="focus-main"><div class="workspace-title"><div><span class="eyebrow">Espace de travail</span><h1>Le code, au premier plan.</h1></div>${button('inspector', showInspector ? 'Masquer l’explication' : 'Afficher l’explication')}</div><div class="focus-workspace ${showInspector ? '' : 'expanded'}"><section class="editor">${tabs()}${code(s)}<div class="editor-bottom"><span>UTF-8 · NASM</span><span>${s.done ? 'Exécution terminée' : 'Suivi de RIP actif'}</span></div></section>${showInspector ? `<aside class="focus-aside">${explanation(s)}<div class="panel-label">État machine <span>hex / décimal</span></div>${registers(s)}<div class="last-step"><span class="eyebrow">Dernière instruction</span><code>${s.last}</code></div></aside>` : ''}</div><div class="focus-bottom">${controls(s)}<span class="muted">${s.done ? 'Code de sortie : 15' : 'Avancez pour observer le prochain effet.'}</span></div></main>`;
}
function quiz(s) {
  if (s.pc !== 4 || s.done) return `<div class="exercise-card"><span class="eyebrow">Observer</span><h3>${s.done ? 'La somme vaut 15.' : 'Une instruction à la fois.'}</h3><p>${s.done ? 'Elle a été copiée dans RDI avant que RAX reçoive le numéro de exit.' : 'Au prochain saut conditionnel, prévoyez le chemin avant d’avancer.'}</p></div>`;
  const correct = answer === (s.zf ? 'exit' : 'loop');
  return `<div class="exercise-card"><span class="eyebrow">À vous de prévoir</span><h3>Où ira le prochain pas ?</h3><p>RCX vaut ${s.rcx}. ZF vaut ${s.zf}.</p><div class="choices">${button('predict-loop', 'Revenir à boucle', answer === 'loop' ? 'chosen' : '')}${button('predict-exit', 'Sortir de la boucle', answer === 'exit' ? 'chosen' : '')}</div>${answer ? `<p class="feedback" role="status">${correct ? 'Exact.' : 'Regardez ZF.'} JNZ saute si ZF = 0. ${s.zf ? 'La boucle se termine.' : 'L’addition sera exécutée à nouveau.'}</p>` : ''}</div>`;
}
function carnet(s) {
  return `<main class="carnet-main"><aside class="chapter"><span class="eyebrow">Carnet d’assembleur</span><div class="chapter-number">03</div><h2>Répéter.<br>Accumuler.<br>Comprendre.</h2><div class="chapter-index"><span>01 · Le compteur</span><span>02 · L’addition</span><strong>03 · Le saut</strong></div><p>Un même programme, trois idées à relier.</p><div class="objective"><span class="eyebrow">Le défi</span><p>Expliquer pourquoi la boucle s’arrête, sans compter les tours à la main.</p></div></aside><section class="lesson"><div class="lesson-heading"><span class="eyebrow">Les boucles / Expérience guidée</span><h1>Cinq tours.<br>Une règle.</h1><p>Lisez l’instruction, anticipez son effet, puis vérifiez.</p></div><div class="lesson-grid"><section class="editor">${tabs()}${code(s)}</section><div class="lesson-notes">${quiz(s)}${explanation(s)}</div></div><div class="lesson-values"><div><span>Somme · RAX</span><strong>${s.rax}</strong></div><div><span>Compteur · RCX</span><strong>${s.rcx}</strong></div><div><span>Zéro · ZF</span><strong>${s.zf}</strong></div></div><div class="lesson-controls">${controls(s)}</div></section></main>`;
}
function stateCard(s, label, step, compare) {
  return `<section class="state-card"><div class="state-heading"><span>${label}</span>${pill('Pas ' + step)}</div><div class="state-instruction"><span class="eyebrow">${s.done ? 'État final' : 'Prochaine instruction'}</span><h2><code>${s.done ? 'exit(15)' : ops[s.pc]}</code></h2></div><div class="state-values">${['rax', 'rcx', 'rdi', 'zf'].map(reg => `<div class="${compare && compare[reg] !== s[reg] ? 'changed' : ''}"><span>${reg.toUpperCase()}</span><strong>${s[reg]}</strong><code>${hex(s[reg])}</code></div>`).join('')}</div><div class="state-foot">RIP <code>${hex(rip(s))}</code></div></section>`;
}
function delta(s) {
  const ref = states[reference];
  const differences = ['rax', 'rcx', 'rdi', 'zf'].filter(reg => ref[reg] !== s[reg]);
  return `<main class="delta-main"><div class="workspace-title"><div><span class="eyebrow">Comparer deux instants</span><h1>Qu’est-ce qui a changé ?</h1><p>Figez un état, puis parcourez l’exécution.</p></div>${button('pin', 'Épingler le pas ' + cursor, 'primary')}</div><div class="delta-grid"><aside class="delta-source"><div class="panel-label">Programme</div>${code(s, true)}<div class="reference-note"><span class="pin-dot"></span>Référence conservée au pas ${reference}</div></aside><section class="comparison"><div class="state-pair">${stateCard(ref, 'Référence épinglée', reference)}${stateCard(s, 'État courant', cursor, ref)}</div><div class="diff-summary"><span class="eyebrow">Différences observées</span>${differences.length ? differences.map(reg => `<div><strong>${reg.toUpperCase()}</strong><code>${ref[reg]} → ${s[reg]}</code><span>${s[reg] - ref[reg] > 0 ? '+' : ''}${s[reg] - ref[reg]}</span></div>`).join('') : '<p>Aucune différence sur RAX, RCX, RDI et ZF.</p>'}<div><strong>RIP</strong><code>${hex(rip(ref))} → ${hex(rip(s))}</code></div></div></section></div>${timeline()}<div class="delta-controls">${controls(s)}<span class="muted">Les valeurs surlignées diffèrent de la référence.</span></div></main>`;
}
function prisme(s) {
  const pc = follow ? s.pc : selected;
  const instructionBytes = bytes[pc].split(' ');
  const descriptions = ['Charge la constante 5 dans ECX.', 'Met EAX à zéro.', 'Additionne RCX à RAX.', 'Décrémente RCX.', 'Saute à 00401007 si ZF = 0.', 'Copie EAX dans EDI.', 'Place le numéro de exit dans EAX.', 'Entre dans l’appel système.'];
  return `<main class="prisme-main"><div class="workspace-title"><div><span class="eyebrow">Établi d’analyse / somme.elf</span><h1>Relier les vues.</h1></div>${button('follow', follow ? '✓ Suivre RIP' : 'Reprendre le suivi RIP', follow ? 'active' : '')}</div><div class="prisme-grid"><aside class="binary-map"><div class="panel-label">Image ELF64</div><div class="binary-file"><span class="file-glyph">ELF</span><strong>somme.elf</strong><span>x86-64 · statique</span></div><div class="section-map"><span>.text</span><strong>24 octets</strong><span>Lecture · Exécution</span></div><div class="binary-facts"><div><span>Point d’entrée</span><code>00401000</code></div><div><span>Imports</span><strong>0</strong></div><div><span>Appel système</span><strong>exit</strong></div><div><span>Chaînes</span><strong>0</strong></div></div><p class="muted">Sélectionnez une instruction pour retrouver ses octets.</p></aside><section class="prisme-list"><div class="panel-label">Désassemblage <span>8 instructions</span></div><div class="listing-table"><div class="listing-header"><span>Adresse</span><span>Octets</span><span>Instruction</span></div>${ops.map((op, i) => `<button id="select-${i}" data-action="select-${i}" class="listing-row ${pc === i ? 'selected' : ''}" aria-pressed="${pc === i}"><code>${hex(addresses[i])}</code><code class="muted">${bytes[i]}</code><code>${op}</code>${!s.done && s.pc === i ? '<span class="rip-marker">RIP</span>' : ''}</button>`).join('')}</div><div class="hex-detail"><div class="eyebrow">Octets sélectionnés / .text + 0x${(addresses[pc] - addresses[0]).toString(16)}</div><div class="byte-strip">${instructionBytes.map(byte => `<code>${byte}</code>`).join('')}</div><p>${instructionBytes.length} octet${instructionBytes.length > 1 ? 's' : ''} · <code>${ops[pc]}</code></p></div><div class="analysis-note"><span class="eyebrow">Lecture statique</span><h3>${pc === 4 ? 'Un branchement vers l’arrière.' : 'Du binaire à l’instruction.'}</h3><p>${descriptions[pc]}</p>${pc === 4 ? '<code>0040100f − 8 → 00401007</code>' : ''}</div></section><aside class="prisme-inspector"><div class="panel-label">Machine <span>Pas ${cursor}</span></div>${registers(s)}${explanation(s)}<div class="inspector-note">${follow ? 'La sélection suit l’exécution.' : 'Sélection manuelle : l’état machine reste au pas ' + cursor + '.'}</div></aside></div><div class="prisme-controls">${controls(s)}<span class="muted">${s.done ? 'Terminé · code de sortie 15' : 'Prochain pas : ' + ops[s.pc]}</span></div></main>`;
}
function render() {
  const focused = document.activeElement?.id;
  const s = states[cursor];
  root.innerHTML = chrome(s) + ({ focus, carnet, delta, prisme })[design](s) + foot(s);
  if (focused) document.getElementById(focused)?.focus({ preventScroll: true });
}
function stop() { if (running !== null) clearInterval(running); running = null; }
function move(n) { stop(); cursor = Math.max(0, Math.min(states.length - 1, cursor + n)); answer = null; render(); }
root.addEventListener('click', event => {
  const id = event.target.closest('[data-action]')?.dataset.action;
  if (!id) return;
  if (id.startsWith('select-')) { selected = Number(id.slice(7)); follow = false; }
  else switch (id) {
    case 'back': move(-1); return;
    case 'step': move(1); return;
    case 'reset': stop(); cursor = 0; answer = null; break;
    case 'run':
      if (running !== null) stop();
      else if (!states[cursor].done) running = setInterval(() => { cursor = Math.min(cursor + 1, states.length - 1); answer = null; if (states[cursor].done) stop(); render(); }, 850);
      break;
    case 'source': case 'listing': view = id; break;
    case 'inspector': showInspector = !showInspector; break;
    case 'pin': reference = cursor; break;
    case 'predict-loop': answer = 'loop'; break;
    case 'predict-exit': answer = 'exit'; break;
    case 'follow': selected = states[cursor].pc; follow = !follow; break;
  }
  render();
});
root.addEventListener('change', event => {
  if (event.target.id === 'time') { stop(); cursor = Number(event.target.value); answer = null; render(); }
});
document.addEventListener('keydown', event => {
  if (event.key === 'F10') { event.preventDefault(); move(event.shiftKey ? -1 : 1); }
});
document.addEventListener('visibilitychange', () => { if (document.hidden) { stop(); render(); } });
render();
