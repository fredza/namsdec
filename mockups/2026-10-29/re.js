'use strict';
const data = RE_DEMO;
const root = document.getElementById('re-app');
const labels = { overview:'Signaux', code:'Code & pseudo-code', graph:'Graphe', strings:'Chaînes', imports:'Imports', protections:'Protections', notes:'Notes' };
let view='code', fn='verify', address='00401183', query='';
const history=[];
const notes = new Map([['00401183','Préfixe ASCII : N puis D. Vérifier si la longueur est contrôlée ailleurs.']]);
const names = new Map();
const escape = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const current = () => data.functions.find(f=>f.id===fn);
const nameOf = f => names.get(f.id) || f.name;
const nav = (target,label,cls='') => `<button data-view="${target}" class="${cls}" ${view===target?'aria-current="page"':''}>${label}</button>`;
const go = (addr,label,cls='') => `<button data-address="${addr}" class="${cls}" ${label==='↗'?'aria-label="Suivre la cible '+addr+'"':''}>${label}</button>`;
function remember(){history.push({view,fn,address});}
function select(addr){
  const target = data.functions.find(f=>f.rows.some(r=>r[0]===addr));
  if(target){remember();fn=target.id;address=addr;view='code';}
  else if(data.strings.some(s=>s.address===addr)){remember();view='strings';address=addr;}
  else if(addr==='00401030'){remember();view='imports';address=addr;}
  render();
}
function functionsList(){
  const results=data.functions.filter(f=>(nameOf(f)+' '+f.address).toLowerCase().includes(query.toLowerCase()));
  return results.length ? results.map(f=>`<button data-function="${f.id}" class="function ${fn===f.id?'selected':''}" aria-pressed="${fn===f.id}"><span class="function-icon">ƒ</span><span><strong>${escape(nameOf(f))}</strong><small>${f.address}</small></span><span class="count">${f.rows.length}</span></button>`).join('') : '<p class="empty">Aucune fonction correspondante.</p>';
}
function sidebar(){return `<aside class="sidebar"><div class="side-label">EXPLORATEUR <span>ELF64</span></div><div class="file"><span class="file-icon">ELF</span><div><strong>sentinel.elf</strong><small>Jeu de démonstration</small></div></div><label class="search"><span>⌕</span><input id="function-search" type="search" placeholder="Filtrer les fonctions…" value="${escape(query)}" aria-label="Filtrer les fonctions"></label><div class="side-label">FONCTIONS <span>2</span></div><div id="functions-list">${functionsList()}</div><div class="side-label secondary-label">EXPLORER</div><div class="side-nav">${nav('overview','◈ <span>Signaux</span><small>3</small>')}${nav('strings','Aa <span>Chaînes</span><small>2</small>')}${nav('imports','↙ <span>Imports</span><small>1</small>')}${nav('protections','◇ <span>Protections</span>')}${nav('notes','≡ <span>Notes de session</span><small>'+notes.size+'</small>')}</div><div class="side-bottom"><span class="local-dot"></span> Prototype local<p>Navigation interactive sur des données fictives.</p></div></aside>`;}
function heading(title,subtitle){return `<div class="view-heading"><div class="eyebrow">SENTINEL.ELF / ANALYSE STATIQUE</div><h1>${title}</h1><p>${subtitle}</p></div>`;}
function codeView(){
  const f=current();
  const mapping = f.id==='verify'
    ? { '00401185':'00401183', '00401187':'0040118b', '0040118d':'0040118b', '00401190':'0040118b', '00401193':'0040118b', '00401198':'00401196' }
    : { '00401121':'00401120', '00401127':'00401124', '00401134':'00401132', '0040113d':'00401136', '00401144':'00401142', '00401145':'00401142', '00401151':'0040114a', '0040115b':'00401156', '0040115c':'00401156' };
  const mapped = mapping[address] || address;
  return `<div class="code-toolbar"><div><span class="function-badge">ƒ</span><strong>${escape(nameOf(f))}</strong><code>${f.address}</code></div>${nav('graph','Voir le graphe ↗','subtle')}</div><div class="code-pair"><section class="assembly"><div class="pane-title"><span>Désassemblage</span><small>NASM · x86-64</small></div><div class="listing-scroll"><div class="listing-columns"><span>Adresse</span><span>Octets</span><span>Instruction</span></div>${f.rows.map(r=>`<div class="instruction ${r[0]===address?'selected':''}"><button data-address="${r[0]}" class="row-select" aria-pressed="${r[0]===address}"><code class="address">${r[0]}</code><code class="bytes">${r[1]}</code><code class="operation">${escape(r[2].replace('verify_token',nameOf(data.functions[1])))}</code><span class="note-dot">${notes.has(r[0])?'●':''}</span></button>${r[3]?go(r[3],'↗','jump'):''}</div>`).join('')}</div><div class="pane-foot">${f.rows.length} instructions · cliquer une ligne pour l’inspecter</div></section><section class="pseudo"><div class="pane-title"><span>Pseudo-code</span><small>Illustration manuelle</small></div><div class="pseudo-lines">${f.pseudo.map((r,i)=>`<button class="pseudo-row ${r[0]===mapped?'selected':''}" data-address="${r[0]}"><span>${i+1}</span><code>${escape(r[1].replace('verify_token',nameOf(data.functions[1])))}</code></button>`).join('')}</div><div class="pseudo-comment"><span class="eyebrow">PISTE DE LECTURE</span><p>${f.description}</p>${f.id==='verify'?'<p>La validation porte sur un <strong>préfixe</strong>. Aucun contrôle de longueur n’apparaît dans cette fonction.</p>':''}</div><div class="engine-note"><span class="engine-dot"></span>Aucun moteur lancé dans la maquette.<br>Une version complète pourrait afficher ici la sortie de DZF ou d’un décompilateur configuré.</div></section></div>`;
}
function overview(){return `${heading('Des faits. Des pistes.','Chaque indice mène à son origine dans le code.')}
  <div class="signal-list"><article><span class="signal-number">01</span><div><span class="tag">COMPARAISONS</span><h2>Deux constantes, deux caractères.</h2><p>0x4e et 0x44 sont comparés aux premiers octets de l’entrée. Ils correspondent à N et D en ASCII.</p>${go('00401183','Examiner verify_token →','action-link')}</div></article><article><span class="signal-number">02</span><div><span class="tag">CHAÎNES ET RÉFÉRENCES</span><h2>Deux issues visibles.</h2><p>« Access granted » et « Access denied » sont référencés par main. Les messages permettent de retrouver les branches.</p>${nav('strings','Voir les chaînes et leurs références →','action-link')}</div></article><article><span class="signal-number">03</span><div><span class="tag">IMPORT</span><h2>Une sortie vers la console.</h2><p>Deux appels passent par puts@plt. Le jeu de démonstration ne décrit aucun import réseau.</p>${nav('imports','Examiner puts →','action-link')}</div></article></div><div class="scope-note">Ces constats décrivent uniquement ce scénario fictif. Ils ne constituent pas un verdict sur un binaire réel.</div>`;}
function graph(){
  const verify=fn==='verify';
  return `${heading('Suivre les branches.',escape(nameOf(current()))+' · cliquer un bloc pour rejoindre son désassemblage.')}
  <div class="graph-canvas"><div class="graph-lane">${go(verify?'00401183':'00401124',`<span class="block-label">${verify?'00401180 · Entrée':'00401120 · Entrée'}</span><code>${verify?"cmp al, 0x4e":"cmp edi, 2"}</code><span>${verify?"Le premier octet vaut-il N ?":"Au moins un argument ?"}</span>`,'graph-block')}<div class="graph-stem"></div><div class="branch-labels"><span>${verify?'Égal · ZF = 1':'argc ≥ 2'}</span><span>${verify?'Différent · ZF = 0':'argc < 2'}</span></div><div class="graph-branches">${go(verify?'0040118b':'0040112d',`<span class="block-label">${verify?'00401187 · Comparer':'00401129 · Vérifier'}</span><code>${verify?'cmp al, 0x44':'call '+escape(nameOf(data.functions[1]))}</code><span>${verify?'Renvoyer input[1] == D':'Tester le résultat retourné'}</span>`,'graph-block success')}${go(verify?'00401196':'0040114a',`<span class="block-label">${verify?'00401196 · Refuser':'0040114a · Refuser'}</span><code>${verify?'xor eax, eax':'puts("Access denied")'}</code><span>${verify?'Renvoyer false':'Renvoyer 1'}</span>`,'graph-block failure')}</div>${verify?'':`<div class="branch-labels"><span>Résultat non nul ↓</span><span>Résultat nul → refus</span></div>${go('00401136','<span class="block-label">00401136 · Accepter</span><code>puts("Access granted")</code><span>Renvoyer 0</span>','graph-block success bottom-block')}`}</div></div>`;
}
function strings(){return `${heading('Remonter depuis les mots.','Les références conduisent aux instructions qui utilisent chaque chaîne.')}<div class="table-scroll"><table><thead><tr><th>Adresse</th><th>Chaîne ASCII</th><th>Référence dans main</th></tr></thead><tbody>${data.strings.map(s=>`<tr class="${address===s.address?'highlight':''}"><td><code>${s.address}</code></td><td><code>"${s.text}"</code></td><td>${go(s.ref,s.ref+' ↗','address-link')}</td></tr>`).join('')}</tbody></table></div><div class="scope-note">2 chaînes illustrées dans .rodata. La recherche de chaînes dans un fichier réel n’est pas exécutée.</div>`;}
function imports(){return `${heading('Les appels vers l’extérieur.','Une importation illustrée dans ce scénario.')}<article class="import-card"><span class="tag">LIBC · SORTIE STANDARD</span><h2>puts</h2><code>int puts(const char *s)</code><p>Affiche une chaîne suivie d’un retour à la ligne. RDI porte le pointeur vers la chaîne.</p><div class="import-refs"><span>Références</span>${go('0040113d','0040113d · chemin accepté ↗','address-link')}${go('00401151','00401151 · chemin refusé ↗','address-link')}</div></article>`;}
function protections(){return `${heading('Lire les protections.','États fictifs choisis pour montrer la présentation ; aucun ELF n’a été inspecté.')}<div class="protection-list">${[
  ['NX','Actif','La pile est décrite comme non exécutable dans cet exemple.'],
  ['PIE','Absent','L’exécutable de cet exemple utilise des adresses fixes.'],
  ['Canari de pile','Non observé','Aucun appel à __stack_chk_fail n’est représenté dans les fonctions illustrées.'],
  ['RELRO','Non renseigné','Les en-têtes dynamiques ne font pas partie du jeu de données.']
].map(([name,state,description])=>`<div><strong>${name}</strong><span class="protection-state">${state}</span><p>${description}</p></div>`).join('')}</div><div class="scope-note">Une protection absente ne prouve pas à elle seule qu’une vulnérabilité est exploitable.</div>`;}
function notesView(){return `${heading('Garder le fil de l’analyse.','Notes associées à des adresses, conservées dans cette page jusqu’à son rechargement.')}<div class="notes-list">${notes.size?[...notes].map(([addr,text])=>`<article>${go(addr,addr+' ↗','address-link')}<p>${escape(text).replace(/\n/g,'<br>')}</p></article>`).join(''):'<p class="empty">Sélectionnez une instruction pour ajouter une note.</p>'}</div>`;}
function inspector(){
  const f=current(), row=f.rows.find(r=>r[0]===address);
  const refs=data.functions.flatMap(f=>f.rows.filter(r=>r[3]===address).map(r=>({f,addr:r[0]})));
  return `<aside class="inspector"><div class="pane-title"><span>Inspecteur</span><small>Contexte sélectionné</small></div><div class="inspect-address"><span class="eyebrow">ADRESSE</span><code>${address}</code><span>${row?escape(nameOf(f)):'Référence externe à la fonction'}</span></div>${row?`<div class="inspect-section"><span class="eyebrow">INSTRUCTION</span><h3><code>${escape(row[2].replace('verify_token',nameOf(data.functions[1])))}</code></h3><div class="byte-chips">${row[1].split(' ').map(b=>`<code>${b}</code>`).join('')}</div><p>${data.explanations[address]||'Instruction sélectionnée dans le listing. Le pseudo-code associé est une interprétation illustrative.'}</p>${row[3]?go(row[3],'Suivre la cible '+row[3]+' ↗','address-link'):''}</div>`:''}<div class="inspect-section"><div class="eyebrow">RÉFÉRENCES ENTRANTES</div>${refs.length?refs.map(ref=>go(ref.addr,escape(nameOf(ref.f))+' · '+ref.addr+' ↗','xref')).join(''):'<p>Aucune référence directe représentée à cette adresse.</p>'}${fn==='verify'&&address!=='00401180'?go('0040112d','Appel à cette fonction · 0040112d ↗','xref'):''}</div><div class="inspect-section"><label class="eyebrow" for="note">NOTE À CETTE ADRESSE</label><textarea id="note" rows="4" placeholder="Votre observation…">${escape(notes.get(address)||'')}</textarea><small id="note-status">Mémoire de cette page · export disponible</small></div><form id="rename-form" class="inspect-section"><label class="eyebrow" for="rename">NOM DE LA FONCTION</label><div class="rename-row"><input id="rename" value="${escape(nameOf(f))}" maxlength="48" required pattern="[A-Za-z_][A-Za-z0-9_]*" title="Identifiant : lettres ASCII, chiffres et soulignement ; ne commence pas par un chiffre."><button type="submit" aria-label="Renommer la fonction">✓</button></div></form></aside>`;
}
function render(){
  root.innerHTML=`<header class="topbar"><a class="brand" href="../index.html"><b>n.</b><strong>namsdec</strong></a><span class="workspace-name">Reverse engineering</span><span class="prototype-badge">MAQUETTE</span><a class="gallery" href="../index.html">Les autres pistes ↗</a></header><div class="demo-banner"><span class="demo-dot"></span><strong>sentinel.elf</strong><span>Scénario fictif · aucun fichier analysé · aucun service Rust connecté</span></div><div class="app-layout">${sidebar()}<main><div class="navigation"><button id="back" ${history.length?'':'disabled'} aria-label="Revenir à la vue précédente">←</button><span>sentinel.elf <span class="breadcrumb-sep">/</span> ${view==='strings'?'.rodata':view==='imports'?'.plt':'.text'} <span class="breadcrumb-sep">/</span> <strong>${escape(nameOf(current()))}</strong></span><button id="export" class="export">Exporter les notes ↓</button></div><nav class="workspace-tabs" aria-label="Vues principales">${nav('overview','Signaux')}${nav('code','Code & pseudo-code')}${nav('graph','Graphe')}${nav('notes','Notes')}</nav><div class="workspace-content">${({overview,code:codeView,graph,strings,imports,protections,notes:notesView})[view]()}</div><div class="context-bar"><span>◈ ${labels[view]}</span><span>${address} · ${notes.size} note${notes.size>1?'s':''}</span></div></main>${inspector()}</div><footer><span><i class="local-dot"></i> Données de démonstration dans la page</span><span>NASM x86-64 · ELF64 · interactions locales</span><span>Prototype RE / 08</span></footer>`;
}
function exportNotes(){
  const content=JSON.stringify({demo:true,file:'sentinel.elf',names:Object.fromEntries(names),notes:Object.fromEntries(notes)},null,2);
  const url=URL.createObjectURL(new Blob([content],{type:'application/json'}));
  const link=document.createElement('a');link.href=url;link.download='namsdec-re-notes-demo.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
}
root.addEventListener('click',event=>{
  const button=event.target.closest('button');if(!button)return;
  if(button.dataset.address){select(button.dataset.address);return;}
  if(button.dataset.function){remember();fn=button.dataset.function;address=current().address;view='code';render();return;}
  if(button.dataset.view){remember();view=button.dataset.view;render();return;}
  if(button.id==='back'&&history.length){({view,fn,address}=history.pop());render();}
  if(button.id==='export')exportNotes();
});
root.addEventListener('input',event=>{
  if(event.target.id==='function-search'){query=event.target.value;document.getElementById('functions-list').innerHTML=functionsList();}
  if(event.target.id==='note'){
    const text=event.target.value;if(text.trim())notes.set(address,text);else notes.delete(address);
    document.getElementById('note-status').textContent='Note conservée dans cette page.';
    if(view==='notes')document.querySelector('.workspace-content').innerHTML=notesView();
    document.querySelector('.context-bar').lastElementChild.textContent=address+' · '+notes.size+' note'+(notes.size>1?'s':'');
    document.querySelector('.side-nav [data-view="notes"] small').textContent=notes.size;
  }
});
root.addEventListener('submit',event=>{
  if(event.target.id==='rename-form'){event.preventDefault();const name=document.getElementById('rename').value.trim();if(/^[A-Za-z_][A-Za-z0-9_]{0,47}$/.test(name)){names.set(fn,name);render();}}
});
render();
