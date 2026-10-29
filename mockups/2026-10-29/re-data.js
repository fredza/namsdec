'use strict';
// Données fictives écrites pour la maquette, pas le résultat d’une analyse.
globalThis.RE_DEMO = {
  functions: [
    { id:'main', name:'main', address:'00401120', description:'Lit le premier argument et choisit le message de sortie.', rows:[
      ['00401120','55','push rbp'], ['00401121','48 89 e5','mov rbp, rsp'],
      ['00401124','83 ff 02','cmp edi, 2'], ['00401127','7c 21','jl 0x40114a','0040114a'],
      ['00401129','48 8b 7e 08','mov rdi, [rsi + 8]'], ['0040112d','e8 4e 00 00 00','call verify_token','00401180'],
      ['00401132','85 c0','test eax, eax'], ['00401134','74 14','je 0x40114a','0040114a'],
      ['00401136','48 8d 3d c7 0e 00 00','lea rdi, [rel str.accepted]','00402004'], ['0040113d','e8 ee fe ff ff','call puts@plt','00401030'],
      ['00401142','31 c0','xor eax, eax'], ['00401144','5d','pop rbp'], ['00401145','c3','ret'],
      ['0040114a','48 8d 3d c4 0e 00 00','lea rdi, [rel str.denied]','00402015'], ['00401151','e8 da fe ff ff','call puts@plt','00401030'],
      ['00401156','b8 01 00 00 00','mov eax, 1'], ['0040115b','5d','pop rbp'], ['0040115c','c3','ret']
    ], pseudo:[
      ['00401120','int main(int argc, char **argv) {'], ['00401124','  if (argc < 2) goto denied;'],
      ['00401129','  char *input = argv[1];'], ['0040112d','  bool accepted = verify_token(input);'],
      ['00401132','  if (!accepted) goto denied;'], ['00401136','  puts("Access granted");'], ['00401142','  return 0;'],
      ['0040114a','denied:'], ['0040114a','  puts("Access denied");'], ['00401156','  return 1;'], ['0040115c','}']
    ]},
    { id:'verify', name:'verify_token', address:'00401180', description:'Compare les deux premiers octets aux caractères N et D.', rows:[
      ['00401180','0f b6 07','movzx eax, byte [rdi]'], ['00401183','3c 4e','cmp al, 0x4e'],
      ['00401185','75 0f','jne 0x401196','00401196'], ['00401187','0f b6 47 01','movzx eax, byte [rdi + 1]'],
      ['0040118b','3c 44','cmp al, 0x44'], ['0040118d','0f 94 c0','sete al'],
      ['00401190','0f b6 c0','movzx eax, al'], ['00401193','c3','ret'],
      ['00401194','90','nop'], ['00401195','90','nop'], ['00401196','31 c0','xor eax, eax'], ['00401198','c3','ret']
    ], pseudo:[
      ['00401180','bool verify_token(const char *input) {'], ['00401183',"  if (input[0] != 'N') {"],
      ['00401196','    return false;'], ['00401185','  }'], ['0040118b',"  return input[1] == 'D';"], ['00401198','}']
    ]}
  ],
  strings:[
    {address:'00402004',text:'Access granted',ref:'00401136'},
    {address:'00402015',text:'Access denied',ref:'0040114a'}
  ],
  explanations:{
    '00401183':'0x4e correspond au caractère ASCII N. Cette comparaison ne modifie pas AL ; elle met à jour les indicateurs.',
    '00401185':'JNE rejoint le retour false lorsque les octets diffèrent. Le déplacement signé est +15 depuis 0x401187.',
    '0040118b':'0x44 correspond au caractère ASCII D. La fonction vérifie un préfixe de deux octets ; elle ne vérifie pas la fin de chaîne.',
    '0040118d':'SETE écrit 1 dans AL si ZF vaut 1, sinon 0. Le résultat vient de la comparaison précédente.',
    '0040112d':'Un appel de main vers verify_token. RDI porte le pointeur vers argv[1] ; EAX recevra le résultat.',
    '00401136':'L’adresse du message de réussite est passée à puts par RDI.',
    '0040114a':'Le chemin de refus affiche le message puis renvoie le code 1.',
    '00401196':'XOR met EAX à zéro : la fonction renvoie false.'
  }
};
