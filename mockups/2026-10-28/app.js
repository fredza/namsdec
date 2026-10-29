/* Three interface studies. Deterministic demonstration, no native execution. */
'use strict';
const design = document.body.dataset.design;
const titles = {atelier:'Atelier guidé', observatoire:'Observatoire', trajectoire:'Trajectoire'};
const instructions = ['mov ecx, 5','xor eax, eax','add rax, rcx','dec rcx','jnz boucle','mov edi, eax','mov eax, 60','syscall'];
const addresses = ['401000','401005','401007','40100a','40100d','40100f','401011','401016'];
const source = [
  ['; La somme des entiers de 1 à 5', 'comment'],
  ['section .text',''], ['global _start',''], ['', ''], ['_start:','label'],
  ['    mov ecx, 5','',0], ['    xor eax, eax','',1], ['', ''], ['boucle:','label'],
  ['    add rax, rcx','',2], ['    dec rcx','',3], ['    jnz boucle','',4], ['', ''],
  ['    mov edi, eax','',5], ['    mov eax, 60','',6], ['    syscall','',7]
];
const states = [{rax:0,rcx:0,rdi:0,pc:0,zf:0,done:false,last:'Début du programme',detail:'Aucune instruction exécutée.'}];
while (!states.at(-1).done) {
  const before = states.at(-1), next = {...before};
  next.last = instructions[before.pc];
  switch(before.pc){
    case 0: next.rcx=5;next.pc=1;next.detail='RCX : 0 → 5';break;
    case 1: next.rax=0;next.zf=1;next.pc=2;next.detail='RAX : 0 → 0 · ZF : 0 → 1';break;
    case 2: next.rax+=next.rcx;next.zf=Number(next.rax===0);next.pc=3;next.detail=`RAX : ${before.rax} → ${next.rax}`;break;
    case 3: next.rcx--;next.zf=Number(next.rcx===0);next.pc=4;next.detail=`RCX : ${before.rcx} → ${next.rcx} · ZF = ${next.zf}`;break;
    case 4: next.pc=next.zf?5:2;next.detail=next.zf?'ZF = 1 · sortie de boucle':'ZF = 0 · retour à boucle';break;
    case 5: next.rdi=next.rax;next.pc=6;next.detail=`RDI : 0 → ${next.rdi}`;break;
    case 6: next.rax=60;next.pc=7;next.detail=`RAX : ${before.rax} → 60`;break;
    case 7: next.done=true;next.detail=`exit(${next.rdi}) · programme terminé`;break;
  }
  states.push(next);
}
let cursor=4, timer=null, activeTab='source', inspector='cpu', consoleTab='console';
const hex=n=>n.toString(16).padStart(8,'0');
const root=document.getElementById('app');
const btn=(action,label,cls='')=>`<button type="button" class="${cls}" data-action="${action}">${label}</button>`;
function controls(){return `<div class="actions">${btn('back','← Reculer')}${btn('step','Un pas → <kbd>F10</kbd>','primary')}${btn('run','▶ Exécuter')}${btn('reset','↺ Repartir')}</div>`;}
function header(){return `<header class="topbar"><div class="brand"><span class="brand-mark">n.</span>namsdec</div><span class="title">${titles[design]}</span><div class="right"><span class="badge">Maquette interactive · ${design==='atelier'?'01':design==='observatoire'?'02':'03'}</span><a class="backlink" href="index.html">← Les trois pistes</a></div></header>`;}
function footer(){return `<footer class="statusbar"><span><b class="run-status">En pause</b> <span class="status-detail">· somme.asm</span></span><span>Démonstration locale · NASM x86-64 · aucune exécution native</span></footer>`;}
function workbar(){return `<div class="workbar"><div class="filetitle"><span class="dot"></span><span class="mono">somme.asm</span><span class="muted" style="font-size:11px;font-weight:400">ELF64</span></div>${controls()}</div>`;}
function tabs(items,current,kind){return `<div class="tabs" role="tablist" aria-label="${kind==='view'?'Vue du programme':kind==='inspect'?'État de la machine':'Journal'}">${items.map(([id,label])=>`<button type="button" role="tab" aria-selected="${current===id}" data-${kind}="${id}">${label}</button>`).join('')}</div>`;}
function editor(){return `<section class="panel editor-panel">${tabs([['source','Source'],['disasm','Désassemblage'],['analysis','Analyse']],activeTab,'view')}<div id="editor-content" role="tabpanel"></div><div class="editorfoot"><span>NASM · x86-64</span><span id="line-status"></span></div></section>`;}
function explain(){return `<section class="explanation"><div class="eyebrow">Prochaine instruction</div><div class="instruction mono" id="next-instruction"></div><p id="next-description"></p><div class="equation" id="next-equation"></div><div style="margin-top:18px">${btn('help','Comprendre cette instruction','textbutton')}</div></section>`;}
function machine(){return `<section class="panel machine-panel">${tabs([['cpu','Registres'],['memory','Mémoire'],['stack','Pile']],inspector,'inspect')}<div id="machine-content" role="tabpanel"></div></section>`;}
function consolePanel(){return `<section class="panel console">${tabs([['console','Console'],['trace','Trace'],['diagnostics','Diagnostics']],consoleTab,'journal')}<div class="consolebody" id="console-content" role="tabpanel"></div></section>`;}
function graph(){return `<section class="panel graph-panel"><div class="panelhead"><span>Déroulement du programme</span><span class="muted">3 blocs · 1 boucle</span></div><div class="graph"><div class="graph-track"><div class="node" data-node="init"><div class="node-title"><span>01 / Initialiser</span><span class="mono">401000</span></div><code>mov ecx, 5<br>xor eax, eax</code></div><div class="connector"></div><div class="node" data-node="loop"><div class="node-title"><span>02 / Accumuler</span><span class="mono">401007</span></div><code data-pc="2">add rax, rcx</code><code data-pc="3">dec rcx</code><code data-pc="4">jnz boucle</code></div><div class="loop-edge"><span id="loop-label">ZF = 0</span></div><div class="connector"><span class="edge-label">ZF = 1 · sortir</span></div><div class="node" data-node="exit"><div class="node-title"><span>03 / Terminer</span><span class="mono">40100f</span></div><code>mov edi, eax<br>mov eax, 60<br>syscall</code></div></div></div><div class="graph-legend"><strong>▣ Bloc actif</strong><span>↩ Saut conditionnel</span><span>↓ Suite du programme</span></div></section>`;}
function atelier(){return `<div class="atelier-shell mainarea"><aside class="lesson-nav"><div class="eyebrow">Votre atelier</div><h2>Comprendre<br>la machine.</h2><ol><li><span class="lesson-index">01</span>Les registres</li><li><span class="lesson-index">02</span>Les opérations</li><li class="current"><span class="lesson-index">03</span>Les boucles</li><li><span class="lesson-index">04</span>La mémoire</li><li><span class="lesson-index">05</span>La pile</li></ol><div class="lesson-bottom"><strong>Un pas. Une explication.</strong>Observez les valeurs, puis avancez à votre rythme.<br><br>${btn('shortcuts','Raccourcis clavier','textbutton')}</div></aside><main class="atelier-main"><div class="intro"><div><div class="eyebrow">Exercice 03 / Les boucles</div><h1>Cinq nombres, une somme.</h1><p>Suivez ce qui change à chaque tour de boucle.</p></div><span class="badge">Parcours découverte</span></div>${workbar()}<div class="atelier-grid"><div>${editor()}${consolePanel()}</div><aside class="stack"><div class="panel">${explain()}</div>${machine()}<div class="goal"><strong>Votre objectif</strong>Comprendre comment RAX atteint 15 et pourquoi la boucle s’arrête.<div class="goal-progress"><i id="progress-fill"></i></div></div></aside></div></main></div>`;}
function observatoire(){return `${workbar()}<main class="expert-grid mainarea"><aside class="expert-nav"><h2>Image chargée</h2><div class="tree-row selected"><span>▾ somme.elf</span><small>24 o</small></div><div class="tree-row indent"><span>.text</span><small>R-X</small></div><h2>Symboles</h2><div class="tree-row"><span class="mono">_start</span><small>401000</small></div><div class="tree-row selected"><span class="mono">boucle</span><small>401007</small></div><h2>Format</h2><div class="binary-meta"><div>Type<b>ELF64</b></div><div>Machine<b>x86-64</b></div><div>Liaison<b>Statique</b></div><div>Imports<b>Aucun</b></div></div><h2>Repères d’analyse</h2><div class="tree-row"><span>1 saut conditionnel</span></div><div class="tree-row"><span>1 appel système</span></div><div class="tree-row"><span>0 chaîne de texte</span></div></aside><section class="expert-main"><div class="space-label"><h2>Listing</h2><span class="eyebrow">Session locale</span></div>${editor()}${consolePanel()}<p class="context-note">Source, octets et état machine restent liés à la même instruction.</p></section><aside class="expert-aside"><div class="eyebrow">Inspecteur d’exécution</div>${explain()}${machine()}<div class="panelhead" style="margin-top:20px"><span>Dernier pas</span><span class="muted" id="last-number"></span></div><div class="mini-list"><code id="last-instruction"></code><span id="last-detail"></span></div></aside></main>`;}
function trajectoire(){return `<main class="trajectory-main mainarea"><div class="trajectory-title"><div><div class="eyebrow">somme.asm / Vue d’exécution</div><h1>Voir le chemin, comprendre le résultat.</h1><p>Le déroulement, les valeurs et le temps dans une même vue.</p></div><span class="badge">Suivi de RIP actif</span></div><div class="trajectory-grid">${graph()}<aside class="stack"><div class="panel">${explain()}</div><section class="panel"><div class="panelhead"><span>L’état au pas sélectionné</span><span class="muted" id="last-number"></span></div><div class="changes"><div class="change"><span class="eyebrow">RAX / accumulateur</span><strong class="value" id="rax-value"></strong><small id="rax-before"></small></div><div class="change"><span class="eyebrow">RCX / compteur</span><strong class="value" id="rcx-value"></strong><small id="rcx-before"></small></div></div><div class="mini-list"><span class="eyebrow">Dernière instruction</span><code id="last-instruction"></code><span id="last-detail"></span></div></section></aside></div><section class="panel timeline-panel"><div class="panelhead"><span>Chronologie</span><span class="muted">Un état après chaque instruction</span></div><div class="timeline-wrap"><label class="eyebrow" for="timeline">Position dans la démonstration</label><input id="timeline" type="range" min="0" max="${states.length-1}" value="${cursor}" aria-label="Pas d’exécution"><div class="timeline-labels"><span>0 · Départ</span><span id="timeline-label"></span><span>${states.length-1} · Fin</span></div><div class="timeline-bottom">${controls()}<span class="timeline-summary" id="timeline-summary"></span></div></div></section>${consolePanel()}</main>`;}
root.innerHTML=`<div class="${design}">${header()}<div class="desktop-note">Disposition adaptée à l’écran. La vue complète se découvre sur ordinateur.</div>${({atelier,observatoire,trajectoire})[design]()}${footer()}</div><dialog id="help-dialog"><div class="eyebrow">Documentation</div><h2 id="help-title"></h2><div id="help-body"></div><div class="dialog-actions">${btn('close-help','Fermer','primary')}</div></dialog>`;
function description(s){
 if(s.done)return ['Programme terminé',`L’appel système exit a reçu ${s.rdi}. Ce programme renvoie la somme comme code de sortie ; il n’écrit rien dans la console.`,`exit(${s.rdi})`];
 switch(s.pc){
 case 0:return [instructions[0],'Initialise le compteur à 5. Écrire dans ECX remet aussi à zéro la moitié haute de RCX.','RCX ← 5'];
 case 1:return [instructions[1],'Met l’accumulateur à zéro. XOR avec soi-même produit zéro et active ZF.','RAX ← 0 · ZF ← 1'];
 case 2:return [instructions[2],'Ajoute le compteur à la somme en cours. RCX reste inchangé ; le résultat est écrit dans RAX.',`${s.rax} + ${s.rcx} → ${s.rax+s.rcx}`];
 case 3:return [instructions[3],'Retire 1 au compteur. Si le résultat est zéro, ZF passe à 1. Le prochain saut utilisera cet indicateur.',`${s.rcx} − 1 → ${s.rcx-1}`];
 case 4:return [instructions[4],s.zf?'ZF vaut 1 : le saut ne sera pas pris. L’exécution sort de la boucle.':'ZF vaut 0 : le saut sera pris. L’exécution revient à l’addition, au début de boucle.',`ZF = ${s.zf} → ${s.zf?'sortir':'recommencer'}`];
 case 5:return [instructions[5],'Copie la somme dans le premier argument de l’appel système exit. RAX sera ensuite utilisé pour son numéro.',`RDI ← ${s.rax}`];
 case 6:return [instructions[6],'Prépare l’appel système Linux exit, dont le numéro est 60. La somme reste conservée dans RDI.','RAX ← 60'];
 default:return [instructions[7],`Termine le programme avec le code de sortie ${s.rdi}. Aucun texte n’est envoyé à la sortie standard.`,`exit(${s.rdi})`];
 }
}
function renderEditor(s){
 const el=document.getElementById('editor-content');if(!el)return;
 if(activeTab==='analysis'){
 el.innerHTML='<div style="padding:22px"><div class="eyebrow">Lecture statique / Programme de démonstration</div><h2 style="font-size:21px;font-weight:500">Une boucle, aucun appel externe.</h2><div class="signal"><strong>Déroulement simple</strong>Trois blocs : initialisation, accumulation, terminaison.<br><small>Le saut à 0x40100d revient à 0x401007 tant que ZF vaut 0.</small></div><div class="signal"><strong>Appel système identifié : exit</strong>RAX reçoit 60 avant syscall. RDI porte le résultat.<br><small>Pas d’écriture sur la sortie standard.</small></div><div class="signal"><strong>Code et données</strong>24 octets de code, aucun import ni chaîne de caractères.<br><small>Ces constats décrivent cet exemple, pas un verdict de sécurité.</small></div></div>';return;
 }
 const disasm=activeTab==='disasm';
 const rows=disasm?instructions.map((i,pc)=>[i,'',pc]):source;
 el.innerHTML=`<div class="code mono">${rows.map(([text,cls,pc],i)=>{
 const current=pc===s.pc&&!s.done;
 const highlighted=cls?`<span class="${cls}">${text||' '}</span>`:text.replace(/\b(mov|xor|add|dec|jnz|syscall|section|global)\b/g,'<span class="op">$1</span>');
 return `<div class="line ${current?'active':''}" ${current?'aria-current="step"':''}><span class="arrow">${current?'▶':''}</span><span class="num">${i+1}</span>${design==='observatoire'?`<span class="address">${pc!==undefined?addresses[pc]:''}</span>`:''}<span>${disasm&&design!=='observatoire'?`<span class="address">${addresses[pc]}  </span>`:''}${highlighted||' '}</span></div>`;
 }).join('')}</div>`;
}
function renderMachine(s){
 const el=document.getElementById('machine-content');if(!el)return;
 if(inspector==='cpu'){
 el.innerHTML=`<div class="registers">${[['RAX',s.rax],['RCX',s.rcx],['RDI',s.rdi]].map(([name,value])=>`<div class="register"><span class="mono">${name}</span><span class="hex mono">${hex(value)}</span><strong>${value}</strong></div>`).join('')}<div class="register"><span class="mono">RIP</span><span class="hex mono">0x${s.done?'401018':addresses[s.pc]}</span><span></span></div></div><div class="flags"><span class="flag ${s.zf?'on':''}">ZF ${s.zf}</span><span class="flag">CF 0</span><span class="flag">SF 0</span><span class="flag">OF 0</span></div>`;
 }else if(inspector==='memory'){
 el.innerHTML='<div class="memory"><span class="eyebrow">.text / Lecture · Exécution</span><pre>401000  b9 05 00 00 00 31 c0 48\n401008  01 c8 48 ff c9 75 f8 89\n401010  c7 b8 3c 00 00 00 0f 05</pre><p>Les octets du programme. Cette boucle ne lit ni n’écrit de données en mémoire.</p></div>';
 }else{
 el.innerHTML='<div class="memory"><span class="eyebrow">Pile / Aucun appel en cours</span><p>Ce programme n’utilise ni <code>push</code>, ni <code>pop</code>, ni <code>call</code>. Il calcule entièrement dans les registres.</p><p class="muted">La pile initiale du processus n’est pas simulée dans cette maquette.</p></div>';
 }
}
function renderConsole(s){
 const el=document.getElementById('console-content');
 if(consoleTab==='trace'){
 el.innerHTML=cursor?states.slice(Math.max(1,cursor-3),cursor+1).map((state,i)=>`<div class="trace-item"><span class="mono">${Math.max(1,cursor-3)+i}</span><code>${state.last}</code><span class="muted">${state.detail}</span></div>`).join(''):'<span class="empty">La trace apparaîtra après le premier pas.</span>';
 }else if(consoleTab==='diagnostics'){
 el.innerHTML='<span class="result">✓ Exemple préassemblé</span><br><span class="muted">Aucun diagnostic pour ce scénario. La maquette ne lance pas NASM.</span>';
 }else{
 el.innerHTML=`<span class="log-label mono">[session]</span> ${s.done?'Exécution terminée.':'Programme prêt. '+cursor+' instruction'+(cursor>1?'s':'')+' exécutée'+(cursor>1?'s':'')+'.'}<br><span class="log-label mono">[sortie]</span> <span class="muted">Ce programme n’écrit pas sur la sortie standard.</span>${s.done?`<br><span class="log-label mono">[résultat]</span> <span class="result">Code de sortie : ${s.rdi} · somme de 1 à 5 = 15</span>`:''}`;
 }
}
function put(id,text){const el=document.getElementById(id);if(el)el.textContent=text;}
function render(){
 const s=states[cursor], prev=states[Math.max(0,cursor-1)], [instruction,desc,eq]=description(s);
 put('next-instruction',instruction);put('next-description',desc);put('next-equation',eq);
 put('line-status',s.done?'Terminé':`RIP 0x${addresses[s.pc]}`);
 put('last-number',`Pas ${cursor}`);put('last-instruction',s.last);put('last-detail',s.detail);
 put('rax-value',String(s.rax));put('rcx-value',String(s.rcx));
 put('rax-before',cursor?`Avant ce pas : ${prev.rax}`:'État initial');put('rcx-before',cursor?`Avant ce pas : ${prev.rcx}`:'État initial');
 put('timeline-label',`Pas ${cursor} / ${states.length-1}`);put('timeline-summary',s.done?'Terminé · résultat 15':`Dernier effet : ${s.detail}`);
 put('loop-label',`ZF = ${s.zf}`);
 const range=document.getElementById('timeline');if(range)range.value=cursor;
 const progress=document.getElementById('progress-fill');if(progress)progress.style.width=`${cursor/(states.length-1)*100}%`;
 root.querySelectorAll('[data-action="back"]').forEach(el=>el.disabled=cursor===0);
 root.querySelectorAll('[data-action="step"]').forEach(el=>el.disabled=s.done);
 root.querySelectorAll('[data-action="run"]').forEach(el=>{el.textContent=timer?'Ⅱ Pause':'▶ Exécuter';el.disabled=s.done;});
 root.querySelectorAll('.run-status').forEach(el=>el.textContent=s.done?'Terminé':timer?'Exécution…':'En pause');
 root.querySelectorAll('.status-detail').forEach(el=>el.textContent=`· somme.asm · pas ${cursor}`);
 root.querySelectorAll('[data-node]').forEach(el=>el.classList.toggle('active',!s.done&&el.dataset.node===(s.pc<2?'init':s.pc<5?'loop':'exit')));
 root.querySelectorAll('[data-pc]').forEach(el=>el.classList.toggle('current-op',Number(el.dataset.pc)===s.pc&&!s.done));
 renderEditor(s);renderMachine(s);renderConsole(s);
}
function stop(){if(timer){clearInterval(timer);timer=null;}}
function move(step){stop();cursor=Math.min(states.length-1,Math.max(0,cursor+step));render();}
function openHelp(shortcuts=false){
 const s=states[cursor];
 put('help-title',shortcuts?'Piloter la démonstration':s.done?'Lire le résultat':instructions[s.pc]);
 document.getElementById('help-body').innerHTML=shortcuts?'<p><strong>F10</strong> : avancer d’une instruction.<br><strong>Maj + F10</strong> : revenir d’une instruction.<br><strong>Échap</strong> : fermer cette aide.</p><p>Exécuter avance automatiquement ; Pause permet de reprendre la main. Repartir restaure l’état initial.</p>':`<p>${description(s)[1]}</p><p><strong>Dernier effet observé</strong><br>${s.detail}</p><p class="muted">Scénario pédagogique fixe. Les états des trois propositions sont identiques.</p>`;
 document.getElementById('help-dialog').showModal();
}
root.addEventListener('click',event=>{
 const target=event.target.closest('button');if(!target)return;
 if(target.dataset.view){activeTab=target.dataset.view;target.parentElement.querySelectorAll('button').forEach(el=>el.setAttribute('aria-selected',String(el===target)));render();return;}
 if(target.dataset.inspect){inspector=target.dataset.inspect;target.parentElement.querySelectorAll('button').forEach(el=>el.setAttribute('aria-selected',String(el===target)));render();return;}
 if(target.dataset.journal){consoleTab=target.dataset.journal;target.parentElement.querySelectorAll('button').forEach(el=>el.setAttribute('aria-selected',String(el===target)));render();return;}
 switch(target.dataset.action){
 case 'back':move(-1);break;
 case 'step':move(1);break;
 case 'reset':stop();cursor=0;render();break;
 case 'run':if(timer)stop();else timer=setInterval(()=>{cursor++;if(cursor>=states.length-1){cursor=states.length-1;stop();}render();},700);render();break;
 case 'help':openHelp();break;
 case 'shortcuts':openHelp(true);break;
 case 'close-help':document.getElementById('help-dialog').close();break;
 }
});
root.addEventListener('input',event=>{if(event.target.id==='timeline'){stop();cursor=Number(event.target.value);render();}});
root.addEventListener('keydown',event=>{
 const tab=event.target.closest('[role="tab"]');
 if(tab&&['ArrowLeft','ArrowRight','Home','End'].includes(event.key)){
 event.preventDefault();const tabs=[...tab.parentElement.querySelectorAll('[role="tab"]')];let i=tabs.indexOf(tab);
 i=event.key==='Home'?0:event.key==='End'?tabs.length-1:(i+(event.key==='ArrowRight'?1:-1)+tabs.length)%tabs.length;
 tabs[i].focus();tabs[i].click();
 }
});
document.addEventListener('keydown',event=>{if(event.key==='F10'&&!document.getElementById('help-dialog').open){event.preventDefault();move(event.shiftKey?-1:1);}});
document.addEventListener('visibilitychange',()=>{if(document.hidden){stop();render();}});
render();
