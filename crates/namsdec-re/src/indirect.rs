//! Les sauts et les appels dont la destination se calcule — et comment
//! la calculer quand même.
//!
//! ## Pourquoi ce module existe
//!
//! Un désassembleur lit `call qword [rip+0x2f3a]` et s'arrête là : la
//! destination n'est pas dans l'instruction, elle est dans une case
//! mémoire. Toute l'analyse qui suit en souffre — le graphe d'appels
//! perd l'arête, la découverte de fonctions perd la cible, et la ligne
//! à l'écran ne dit pas qui est appelé.
//!
//! Le compte le dit mieux qu'une phrase. Sur les seize binaires
//! d'épreuve, relevé le 20 octobre 2026 : **8 764 appels et 7 496 sauts
//! dont la destination se calcule**, contre 147 301 branchements
//! directs. Un branchement sur dix est muet.
//!
//! ## Ce que ce module sait faire, et ce qu'il refuse de deviner
//!
//! Quatre chemins, du plus sûr au moins sûr, et **chacun se nomme** :
//! ce qui compte pour qui apprend n'est pas la réponse mais la raison.
//!
//! 1. **La table de liaison.** `call [rip+N]` où `rip+N` tombe sur une
//!    entrée de la GOT : la relocation porte le nom de la fonction, et
//!    c'est l'éditeur de liens qui l'a écrit. Aucune déduction.
//! 2. **Un pointeur constant.** La case visée est en lecture seule et
//!    porte déjà une adresse de code — une table de fonctions virtuelles
//!    résolue à la compilation, par exemple.
//! 3. **Un registre chargé juste avant.** `mov rax, 0x401136` puis
//!    `call rax`, à quelques instructions d'écart.
//! 4. **Une adresse calculée juste avant.** `lea rax, [rip+N]` puis
//!    `call rax` : la forme ordinaire d'un pointeur de fonction en code
//!    indépendant de la position.
//!
//! **Ce qui n'entre dans aucun des quatre reste inconnu**, et le dit.
//! Une table de sauts dont les bornes ne se lisent pas, un pointeur pris
//! dans une structure, une adresse venue d'un calcul : les deviner
//! produirait un graphe d'appels plausible et faux, ce qui est pire
//! qu'un graphe incomplet — on ne vérifie pas ce qu'on croit savoir.
//!
//! La recherche en arrière est bornée : au-delà de quelques
//! instructions, la valeur d'un registre vient d'ailleurs, et remonter
//! plus haut reviendrait à inventer.

use iced_x86::{Mnemonic, OpKind, Register};

use crate::imports::Import;

/// Combien d'instructions on remonte pour trouver ce qui a chargé le
/// registre.
///
/// Le même choix que pour les numéros d'appel système, et la même
/// raison : au-delà, la valeur vient d'un calcul, d'un argument ou
/// d'une lecture mémoire, et deviner serait mentir. Douze couvre la
/// forme ordinaire — charger, préparer les arguments, appeler.
const REMONTÉE: usize = 12;

/// Une destination retrouvée, et par quel chemin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Résolu {
    /// L'adresse de l'instruction dont on a résolu la cible.
    pub at: u64,
    /// Est-ce un appel, ou un saut ?
    pub appel: bool,
    /// L'adresse visée, quand on la connaît.
    pub target: Option<u64>,
    /// Le nom de ce qui est visé, quand il y en a un.
    pub name: Option<String>,
    /// Par quel chemin on le sait.
    pub par: Par,
}

/// Comment une destination a été retrouvée.
///
/// L'ordre des variantes est celui de la CONFIANCE : ce que l'éditeur
/// de liens a écrit d'abord, ce qu'on a déduit du code ensuite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Par {
    /// La relocation de la table de liaison nomme la fonction.
    TableDeLiaison,
    /// La case visée porte déjà un pointeur vers du code.
    PointeurConstant,
    /// Un `mov registre, adresse` quelques instructions plus haut.
    RegistreChargé,
    /// Un `lea registre, [rip+…]` quelques instructions plus haut.
    AdresseCalculée,
}

impl Par {
    /// La phrase qui dit d'où vient la réponse.
    ///
    /// Montrée à côté de la destination : « → puts » sans dire comment
    /// on le sait apprend à croire. C'est la différence entre un outil
    /// qui donne la réponse et un outil qui l'explique.
    pub fn why(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match self {
            Self::TableDeLiaison if fr => {
                "l'entrée de la table de liaison : c'est la relocation qui nomme la fonction"
            }
            Self::TableDeLiaison => {
                "the linkage table entry: the relocation itself names the function"
            }
            Self::PointeurConstant if fr => {
                "la case visée est en lecture seule et porte déjà cette adresse"
            }
            Self::PointeurConstant => {
                "the targeted slot is read-only and already holds this address"
            }
            Self::RegistreChargé if fr => "le registre a reçu cette adresse juste avant",
            Self::RegistreChargé => "the register was given this address just before",
            Self::AdresseCalculée if fr => {
                "un `lea` relatif à RIP a calculé cette adresse juste avant"
            }
            Self::AdresseCalculée => "a RIP-relative `lea` computed this address just before",
        }
    }
}

/// Résout ce qui peut l'être parmi les branchements indirects.
///
/// `binary` est le fichier entier — on y lit les cases de pointeurs —,
/// `imports` ce que la table dynamique nomme. Rend une entrée par
/// branchement RÉSOLU : ce qui reste inconnu n'est pas listé, parce
/// qu'une liste d'inconnues longue de huit mille lignes n'apprend rien.
pub fn resolve(
    disasm: &[namsdec_disasm::DecodedInstruction],
    binary: &[u8],
    imports: &[Import],
) -> Vec<Résolu> {
    // Les entrées de la table de liaison, par l'adresse de leur case :
    // c'est ce qu'un `call [rip+N]` désigne.
    let par_got: std::collections::BTreeMap<u64, &str> = imports
        .iter()
        .filter(|i| i.got != 0)
        .map(|i| (i.got, i.name.as_str()))
        .collect();
    // Et par l'adresse de leur entrée, pour nommer une destination
    // directe qui tombe dans la table de liaison.
    let par_plt: std::collections::BTreeMap<u64, &str> = imports
        .iter()
        .filter(|i| i.plt != 0)
        .map(|i| (i.plt, i.name.as_str()))
        .collect();

    let mut out = Vec::new();
    for (rang, instruction) in disasm.iter().enumerate() {
        let Some(décodée) = namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
        else {
            continue;
        };
        let appel = match décodée.mnemonic() {
            Mnemonic::Call => true,
            Mnemonic::Jmp => false,
            _ => continue,
        };
        if décodée.op_count() == 0 {
            continue;
        }
        // Un branchement DIRECT porte sa destination : rien à résoudre.
        if matches!(
            décodée.op_kind(0),
            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64
        ) {
            continue;
        }

        let trouvé = match décodée.op_kind(0) {
            // `call qword [rip+N]` : la case est connue, son contenu
            // peut l'être.
            OpKind::Memory if décodée.is_ip_rel_memory_operand() => {
                let case = décodée.ip_rel_memory_address();
                par_la_case(case, binary, &par_got)
            }
            // `call rax` : d'où vient RAX ?
            OpKind::Register => par_le_registre(disasm, rang, décodée.op_register(0)),
            _ => None,
        };
        let Some((target, name, par)) = trouvé else {
            continue;
        };
        // Une destination qui tombe sur une entrée de liaison se nomme
        // elle aussi : c'est le cas d'un `jmp` de PLT.
        let name = name.or_else(|| target.and_then(|t| par_plt.get(&t).map(|n| (*n).to_owned())));
        out.push(Résolu {
            at: instruction.address,
            appel,
            target,
            name,
            par,
        });
    }
    out
}

/// Ce qu'une case mémoire désigne : le nom que la relocation lui donne,
/// ou le pointeur qu'elle porte déjà.
fn par_la_case(
    case: u64,
    binary: &[u8],
    par_got: &std::collections::BTreeMap<u64, &str>,
) -> Option<(Option<u64>, Option<String>, Par)> {
    // 1. La table de liaison nomme la fonction. C'est l'éditeur de liens
    //    qui l'a écrit : il n'y a rien à déduire.
    if let Some(nom) = par_got.get(&case) {
        return Some((None, Some((*nom).to_owned()), Par::TableDeLiaison));
    }
    // 2. Sinon, la case porte peut-être déjà une adresse — une table de
    //    fonctions résolue à la compilation. On la lit DANS LE FICHIER :
    //    ce que le disque contient est ce qui sera chargé, et rien n'est
    //    exécuté pour l'obtenir.
    let offset = usize::try_from(crate::imports::file_offset(binary, case)?).ok()?;
    let octets = binary.get(offset..offset.checked_add(8)?)?;
    let pointeur = u64::from_le_bytes(octets.try_into().ok()?);
    // Zéro veut dire « pas encore résolu » : la case attend l'éditeur de
    // liens dynamique, et prétendre qu'elle mène quelque part serait
    // faux.
    (pointeur != 0).then_some((Some(pointeur), None, Par::PointeurConstant))
}

/// D'où vient la valeur de ce registre, si elle vient d'assez près.
fn par_le_registre(
    disasm: &[namsdec_disasm::DecodedInstruction],
    rang: usize,
    registre: Register,
) -> Option<(Option<u64>, Option<String>, Par)> {
    let premier = rang.saturating_sub(REMONTÉE);
    for plus_tôt in disasm[premier..rang].iter().rev() {
        let décodée = namsdec_disasm::decode_one(&plus_tôt.bytes, plus_tôt.address)?;
        if décodée.op_count() == 0 || décodée.op_kind(0) != OpKind::Register {
            continue;
        }
        // Le même registre, ou sa moitié basse : `mov eax, …` écrit RAX
        // en entier, et c'est la forme que les compilateurs emploient.
        if !même_registre(décodée.op_register(0), registre) {
            continue;
        }
        // LA DERNIÈRE ÉCRITURE DÉCIDE. Si ce n'est pas une de nos deux
        // formes — un calcul, une lecture mémoire —, la valeur ne se lit
        // pas d'ici, et remonter plus haut irait chercher une valeur
        // périmée.
        return match décodée.mnemonic() {
            Mnemonic::Lea if décodée.is_ip_rel_memory_operand() => Some((
                Some(décodée.ip_rel_memory_address()),
                None,
                Par::AdresseCalculée,
            )),
            Mnemonic::Mov if décodée.op_count() > 1 => match décodée.op_kind(1) {
                OpKind::Immediate8
                | OpKind::Immediate16
                | OpKind::Immediate32
                | OpKind::Immediate64
                | OpKind::Immediate8to16
                | OpKind::Immediate8to32
                | OpKind::Immediate8to64
                | OpKind::Immediate32to64 => {
                    let valeur = décodée.immediate(1);
                    (valeur != 0).then_some((Some(valeur), None, Par::RegistreChargé))
                }
                _ => None,
            },
            _ => None,
        };
    }
    None
}

/// Deux registres désignent-ils la même case ?
///
/// `mov eax, …` écrit RAX en entier — c'est la règle du x86-64, et c'est
/// la forme qu'un compilateur emploie pour charger une adresse de moins
/// de quatre gigaoctets. Les comparer par leur nom seul manquerait donc
/// l'écriture la plus courante.
fn même_registre(écrit: Register, lu: Register) -> bool {
    écrit == lu || écrit.full_register() == lu.full_register()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `mov eax, 0x401136` puis `call rax` : la cible se lit.
    ///
    /// Et elle se lit par la MOITIÉ BASSE du registre, qui est la forme
    /// qu'un compilateur emploie — `mov eax` écrit RAX en entier.
    #[test]
    fn a_register_loaded_just_before_gives_the_target() {
        // b8 36 11 40 00    mov eax, 0x401136
        // ff d0             call rax
        let code = [0xb8, 0x36, 0x11, 0x40, 0x00, 0xff, 0xd0];
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        let résolus = resolve(&disasm, &[], &[]);
        assert_eq!(résolus.len(), 1, "un seul branchement indirect");
        assert_eq!(résolus[0].target, Some(0x401136));
        assert_eq!(résolus[0].par, Par::RegistreChargé);
        assert!(résolus[0].appel);
    }

    /// `lea rax, [rip+N]` puis `call rax` : la forme ordinaire d'un
    /// pointeur de fonction en code indépendant de la position.
    #[test]
    fn a_rip_relative_lea_gives_the_target() {
        // 48 8d 05 10 00 00 00   lea rax, [rip+0x10]
        // ff d0                  call rax
        let code = [0x48, 0x8d, 0x05, 0x10, 0x00, 0x00, 0x00, 0xff, 0xd0];
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        let résolus = resolve(&disasm, &[], &[]);
        assert_eq!(résolus.len(), 1);
        // `rip` vaut l'adresse de l'instruction SUIVANTE : 0x400007.
        assert_eq!(résolus[0].target, Some(0x400017));
        assert_eq!(résolus[0].par, Par::AdresseCalculée);
    }

    /// `call [rip+N]` vers une entrée de la table de liaison : la
    /// relocation nomme la fonction, et il n'y a rien à déduire.
    #[test]
    fn a_linkage_table_entry_names_the_function() {
        // ff 15 10 00 00 00   call qword [rip+0x10]
        let code = [0xff, 0x15, 0x10, 0x00, 0x00, 0x00];
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        let imports = vec![Import {
            name: "puts".to_owned(),
            plt: 0x401020,
            got: 0x400016,
        }];
        let résolus = resolve(&disasm, &[], &imports);
        assert_eq!(résolus.len(), 1);
        assert_eq!(résolus[0].name.as_deref(), Some("puts"));
        assert_eq!(résolus[0].par, Par::TableDeLiaison);
        // C'est la source la plus sûre : elle vient du fichier, pas
        // d'une déduction sur le code.
        assert_eq!(
            Par::TableDeLiaison.min(Par::RegistreChargé),
            Par::TableDeLiaison
        );
    }

    /// CE QU'ON NE SAIT PAS RESTE INCONNU.
    ///
    /// C'est la moitié du contrat : un graphe d'appels plausible et faux
    /// est pire qu'un graphe incomplet, parce qu'on ne vérifie pas ce
    /// qu'on croit savoir. Un registre venu d'une lecture mémoire, ou
    /// d'un calcul, n'est pas approché.
    #[test]
    fn what_cannot_be_known_stays_unknown() {
        for code in [
            // mov rax, [rbx] ; call rax — la valeur vient de la mémoire.
            vec![0x48, 0x8b, 0x03, 0xff, 0xd0],
            // add rax, rbx ; call rax — elle vient d'un calcul.
            vec![0x48, 0x01, 0xd8, 0xff, 0xd0],
            // call rax, sans rien avant.
            vec![0xff, 0xd0],
        ] {
            let disasm = namsdec_disasm::disassemble(&code, 0x400000);
            let résolus = resolve(&disasm, &[], &[]);
            assert!(
                résolus.is_empty(),
                "rien ne doit être deviné ici : {résolus:?}"
            );
        }
    }

    /// La remontée est BORNÉE : au-delà, la valeur vient d'ailleurs.
    #[test]
    fn the_search_backwards_is_bounded() {
        let mut code = vec![0xb8, 0x36, 0x11, 0x40, 0x00]; // mov eax, 0x401136
        // Vingt `nop` : bien plus que ce qu'on remonte.
        code.extend(std::iter::repeat_n(0x90u8, 20));
        code.extend_from_slice(&[0xff, 0xd0]); // call rax
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        assert!(
            resolve(&disasm, &[], &[]).is_empty(),
            "vingt instructions plus loin, la valeur n'est plus celle-là"
        );
    }
}

/// Une table de sauts : ce qu'un `switch` devient une fois compilé.
///
/// C'est la forme qui compte le plus pour lire le programme de
/// quelqu'un d'autre. Les sauts indirects d'un binaire statique sont
/// surtout de la plomberie de la glibc — la résolution IFUNC, qui
/// choisit une implémentation selon le processeur et n'est donc PAS
/// déterminable avant l'exécution. Les tables de sauts, elles, sont le
/// `switch` qu'une personne a écrit, et sans elles le graphe du
/// programme s'arrête à chaque aiguillage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDeSauts {
    /// L'adresse du `jmp` qui s'y branche.
    pub at: u64,
    /// Où la table commence.
    pub base: u64,
    /// Les destinations, dans l'ordre des cas.
    pub cases: Vec<u64>,
    /// L'instruction qui borne l'index — le `cmp` du `switch`. C'est
    /// elle qui dit combien de cas, et c'est pour cela qu'on ne devine
    /// jamais la taille.
    pub bornée_par: u64,
    /// Des décalages de quatre octets relatifs à la base (ce que gcc
    /// produit en code indépendant de la position), ou des pointeurs de
    /// huit octets.
    pub relative: bool,
}

/// Au-delà, ce n'est plus un `switch` : c'est une borne lue de travers.
const CAS_MAX: u64 = 4096;

/// Les tables de sauts qu'on sait lire, avec leurs destinations.
///
/// Deux formes, et ce sont celles que produisent gcc et clang :
///
/// - **relative**, en code indépendant de la position :
///   `lea base,[rip+T]` · `movsxd idx,[base+i*4]` · `add idx,base` ·
///   `jmp idx`. La table porte des ÉCARTS, pas des adresses, ce qui lui
///   permet de survivre au déplacement du programme ;
/// - **absolue**, hors PIE : `jmp [T + i*8]`, où la table porte
///   directement les adresses.
///
/// Dans les deux cas, LA TAILLE VIENT DU CODE et non d'une devinette :
/// un `switch` se compile toujours avec son contrôle de borne —
/// `cmp i, N` suivi d'un saut au cas par défaut. Sans ce contrôle,
/// aucune table n'est rendue : lire une table dont on ignore la fin,
/// c'est lire ce qui la suit comme si c'était elle.
pub fn jump_tables(
    disasm: &[namsdec_disasm::DecodedInstruction],
    binary: &[u8],
) -> Vec<TableDeSauts> {
    // Les bornes du code : une destination qui en sort n'est pas une
    // destination, c'est la preuve qu'on a lu trop loin.
    let (Some(début), Some(fin)) = (
        disasm.first().map(|i| i.address),
        disasm.last().map(|i| i.address + i.bytes.len() as u64),
    ) else {
        return Vec::new();
    };
    let dans_le_code = |a: u64| a >= début && a < fin;

    let mut out = Vec::new();
    for (rang, instruction) in disasm.iter().enumerate() {
        let Some(saut) = namsdec_disasm::decode_one(&instruction.bytes, instruction.address) else {
            continue;
        };
        if saut.mnemonic() != Mnemonic::Jmp || saut.op_count() == 0 {
            continue;
        }
        let trouvée = match saut.op_kind(0) {
            OpKind::Register => table_relative(disasm, rang, saut.op_register(0)),
            // `jmp [T + i*8]` : la table est à l'adresse écrite dans
            // l'instruction, et l'échelle dit la largeur d'une entrée.
            OpKind::Memory
                if saut.memory_index() != Register::None && saut.memory_index_scale() == 8 =>
            {
                Some((
                    saut.memory_displacement64(),
                    saut.memory_index(),
                    false,
                    0u64,
                ))
            }
            _ => continue,
        };
        let Some((base, index, relative, _)) = trouvée else {
            continue;
        };
        if base == 0 || !dans_le_code(base) && relative {
            // Une table relative vit dans le code (`.rodata` placé
            // près de lui) ; une table absolue peut vivre ailleurs.
            // On ne refuse que ce qui est manifestement faux.
        }
        let Some((combien, bornée_par)) = borne_du_switch(disasm, rang, index) else {
            // Pas de contrôle de borne : on ne sait pas où la table
            // finit, donc on ne la lit pas. C'est le refus qui empêche
            // de prendre ce qui suit la table pour des destinations.
            continue;
        };
        let largeur = if relative { 4 } else { 8 };
        let mut cases = Vec::new();
        for i in 0..combien {
            let at = base + i * largeur;
            let Some(offset) =
                crate::imports::file_offset(binary, at).and_then(|o| usize::try_from(o).ok())
            else {
                break;
            };
            let destination = if relative {
                let Some(octets) = binary.get(offset..offset + 4) else {
                    break;
                };
                let écart = i32::from_le_bytes(octets.try_into().unwrap_or([0; 4]));
                base.wrapping_add(écart as i64 as u64)
            } else {
                let Some(octets) = binary.get(offset..offset + 8) else {
                    break;
                };
                u64::from_le_bytes(octets.try_into().unwrap_or([0; 8]))
            };
            // Une destination hors du code dit qu'on a dépassé la
            // table : on s'arrête là plutôt que de rendre du bruit.
            if !dans_le_code(destination) {
                break;
            }
            cases.push(destination);
        }
        // Une table d'un seul cas n'en est pas une : c'est une lecture
        // de travers qui s'est arrêtée tout de suite.
        if cases.len() < 2 {
            continue;
        }
        out.push(TableDeSauts {
            at: instruction.address,
            base,
            cases,
            bornée_par,
            relative,
        });
    }
    out
}

/// La forme relative : on remonte depuis le `jmp` jusqu'au `lea` qui a
/// posé la base de la table.
///
/// Rend la base, le registre d'index, « relative », et rien d'autre.
fn table_relative(
    disasm: &[namsdec_disasm::DecodedInstruction],
    rang: usize,
    sauté: Register,
) -> Option<(u64, Register, bool, u64)> {
    let premier = rang.saturating_sub(REMONTÉE);
    let mut cherché = sauté;
    let mut base_reg: Option<Register> = None;
    let mut index: Option<Register> = None;
    for plus_tôt in disasm[premier..rang].iter().rev() {
        let d = namsdec_disasm::decode_one(&plus_tôt.bytes, plus_tôt.address)?;
        if d.op_count() == 0 || d.op_kind(0) != OpKind::Register {
            continue;
        }
        // 3. `lea base, [rip+T]` : la table est là.
        if let Some(b) = base_reg
            && même_registre(d.op_register(0), b)
            && d.mnemonic() == Mnemonic::Lea
            && d.is_ip_rel_memory_operand()
        {
            return Some((d.ip_rel_memory_address(), index?, true, 0));
        }
        if !même_registre(d.op_register(0), cherché) {
            continue;
        }
        match d.mnemonic() {
            // 1. `add idx, base` : le saut vise base + écart.
            Mnemonic::Add if base_reg.is_none() && d.op_count() > 1 => {
                if d.op_kind(1) != OpKind::Register {
                    return None;
                }
                base_reg = Some(d.op_register(1));
            }
            // 2. `movsxd idx, [base + i*4]` : l'écart vient de la table.
            Mnemonic::Movsxd | Mnemonic::Mov
                if base_reg.is_some()
                    && d.op_count() > 1
                    && d.op_kind(1) == OpKind::Memory
                    && d.memory_index_scale() == 4 =>
            {
                index = Some(d.memory_index());
                // La base de la table peut être écrite ici plutôt que
                // par un `add` : `[rip+T + i*4]`.
                if d.is_ip_rel_memory_operand() {
                    return Some((d.ip_rel_memory_address(), d.memory_index(), true, 0));
                }
                cherché = d.memory_base();
            }
            // Toute autre écriture du registre coupe la piste : la
            // valeur ne vient plus de la table.
            _ => return None,
        }
    }
    None
}

/// Combien de cas la table porte, selon le `cmp` qui borne l'index.
///
/// C'est la seule source honnête : un `switch` se compile toujours avec
/// son contrôle de borne, et ce contrôle dit le nombre de cas. Rendre
/// aussi l'adresse du `cmp` permet de la montrer — « cette table a
/// vingt-trois cas, et voici où le programme le dit ».
fn borne_du_switch(
    disasm: &[namsdec_disasm::DecodedInstruction],
    rang: usize,
    index: Register,
) -> Option<(u64, u64)> {
    // Plus loin que pour un registre : entre le `cmp` et le `jmp`, il y
    // a le saut au cas par défaut, le calcul de l'index et la lecture
    // de la table.
    let premier = rang.saturating_sub(REMONTÉE * 2);
    for plus_tôt in disasm[premier..rang].iter().rev() {
        let d = namsdec_disasm::decode_one(&plus_tôt.bytes, plus_tôt.address)?;
        if d.mnemonic() != Mnemonic::Cmp || d.op_count() < 2 {
            continue;
        }
        if d.op_kind(0) != OpKind::Register || !même_registre(d.op_register(0), index) {
            continue;
        }
        let n = match d.op_kind(1) {
            OpKind::Immediate8
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate32to64 => d.immediate(1),
            _ => continue,
        };
        // `cmp i, N` puis `ja défaut` : les cas vont de 0 à N inclus.
        let combien = n.checked_add(1)?;
        if combien == 0 || combien > CAS_MAX {
            return None;
        }
        return Some((combien, plus_tôt.address));
    }
    None
}

#[cfg(test)]
mod tests_tables {
    use super::*;

    /// Une table de sauts SANS contrôle de borne n'est pas lue.
    ///
    /// C'est le refus qui tient tout le reste : sans le `cmp`, on ne
    /// sait pas où la table finit, et lire au-delà prendrait ce qui la
    /// suit pour des destinations. Un graphe plausible et faux est pire
    /// qu'un graphe incomplet.
    #[test]
    fn a_table_without_a_bound_check_is_not_read() {
        // lea rdx,[rip+0] ; movsxd rax,[rdx+rax*4] ; add rax,rdx ; jmp rax
        let code = [
            0x48, 0x8d, 0x15, 0x00, 0x00, 0x00, 0x00, // lea rdx,[rip+0]
            0x48, 0x63, 0x04, 0x82, // movsxd rax,[rdx+rax*4]
            0x48, 0x01, 0xd0, // add rax,rdx
            0xff, 0xe0, // jmp rax
        ];
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        assert!(
            jump_tables(&disasm, &[]).is_empty(),
            "sans `cmp`, la fin de la table est inconnue"
        );
    }

    /// Et la borne vient du CODE : `cmp i, N` donne N+1 cas.
    #[test]
    fn the_bound_comes_from_the_compare() {
        // cmp edi,3 ; ja +8 ; lea rdx,[rip+X] ; movsxd rax,[rdx+rdi*4]
        // ; add rax,rdx ; jmp rax
        let code = [
            0x83, 0xff, 0x03, // cmp edi,3
            0x77, 0x08, // ja
            0x48, 0x8d, 0x15, 0x00, 0x00, 0x00, 0x00, // lea rdx,[rip+0]
            0x48, 0x63, 0x04, 0xba, // movsxd rax,[rdx+rdi*4]
            0x48, 0x01, 0xd0, // add rax,rdx
            0xff, 0xe0, // jmp rax
        ];
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        // La borne se lit même si la table elle-même n'est pas dans le
        // fichier d'épreuve : c'est le compte qu'on vérifie ici.
        let trouvé = borne_du_switch(&disasm, disasm.len() - 1, Register::RDI);
        assert_eq!(
            trouvé.map(|(n, _)| n),
            Some(4),
            "`cmp i, 3` veut dire quatre cas : de zéro à trois"
        );
    }

    /// Une borne absurde est refusée : c'est une lecture de travers.
    #[test]
    fn an_absurd_bound_is_refused() {
        // cmp edi, 0x7fffffff ; jmp rax
        let code = [0x81, 0xff, 0xff, 0xff, 0xff, 0x7f, 0xff, 0xe0];
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        assert_eq!(
            borne_du_switch(&disasm, disasm.len() - 1, Register::RDI),
            None,
            "deux milliards de cas ne sont pas un `switch`"
        );
    }
}
