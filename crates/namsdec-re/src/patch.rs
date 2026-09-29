//! Changer des octets dans un binaire.
//!
//! Le besoin apparaît de lui-même : on ouvre un crackme, on voit le `je`
//! qu'il faudrait inverser, et l'on ne peut rien en faire. Réassembler le
//! listing est refusé — à juste titre, un désassemblage n'est pas un
//! source. Ce qu'il faut est plus modeste et plus sûr : **écrire les
//! octets** à leur place dans le fichier, et le réécrire tel quel.
//!
//! Deux gestes couvrent presque tous les cas :
//!
//! - **inverser un saut conditionnel** — `je` devient `jne`, et le
//!   programme prend l'autre branche ;
//! - **neutraliser une instruction** — la remplacer par des `nop`, ce
//!   qui supprime un appel de vérification sans déplacer quoi que ce
//!   soit ;
//! - **forcer un saut conditionnel** — `je` devient `jmp`, et le
//!   programme prend *toujours* la branche. Inverser et forcer ne
//!   répondent pas à la même question : inverser demande « et si la
//!   condition était l'autre ? », forcer demande « et si on y allait
//!   quoi qu'il arrive ? ». C'est le second qu'on veut devant un
//!   contrôle qu'on n'a pas encore compris.
//!
//! Les deux gardent la **longueur exacte** de ce qu'ils remplacent.
//! C'est la règle qui rend l'opération sûre : un octet de plus ou de
//! moins décalerait tout ce qui suit, et chaque adresse du fichier
//! deviendrait fausse.

/// Un changement appliqué à un fichier, et de quoi le défaire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    /// Où, dans le fichier.
    pub offset: usize,
    /// Ce qu'il y avait avant. Gardé pour pouvoir revenir en arrière :
    /// un patch qu'on ne peut pas défaire n'est pas un outil d'analyse,
    /// c'est une décision définitive.
    pub before: Vec<u8>,
    /// Ce qu'on a mis à la place. De la même longueur que `before`.
    pub after: Vec<u8>,
    /// L'adresse virtuelle correspondante, pour le dire à l'écran.
    pub addr: u64,
}

/// L'inverse d'un saut conditionnel, s'il en a un.
///
/// La règle tient en un bit, et c'est le processeur qui l'a voulue
/// ainsi : les conditions vont par paires, et le bit de poids faible de
/// l'opcode dit laquelle des deux. `je` est `0x74`, `jne` est `0x75` ;
/// `jb` est `0x72`, `jae` est `0x73`. Un `ou exclusif` avec 1 passe donc
/// de l'une à l'autre, quelle que soit la condition — il n'y a pas de
/// table à tenir à jour.
///
/// Deux formes existent : le saut court (`0x70`–`0x7F`, un octet de
/// déplacement) et le saut long (`0x0F` puis `0x80`–`0x8F`, quatre
/// octets). Les deux se traitent pareil, sur l'octet qui porte la
/// condition.
///
/// Rend `None` sur ce qui n'est pas un saut conditionnel — y compris sur
/// `jrcxz` (`0xE3`), qui n'a pas d'inverse en une instruction. Mieux
/// vaut refuser que d'écrire quelque chose d'approchant.
pub fn inverted_jump(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [opcode, reste @ ..] if (0x70..=0x7f).contains(opcode) => {
            let mut out = vec![opcode ^ 1];
            out.extend_from_slice(reste);
            Some(out)
        }
        [0x0f, opcode, reste @ ..] if (0x80..=0x8f).contains(opcode) => {
            let mut out = vec![0x0f, opcode ^ 1];
            out.extend_from_slice(reste);
            Some(out)
        }
        _ => None,
    }
}

/// Le même saut, mais inconditionnel — `je` devient `jmp`.
///
/// La longueur est gardée, comme pour les deux autres gestes, et c'est
/// ce qui demande un peu de soin :
///
/// * **saut court** (`0x70`–`0x7F`, deux octets) : `jmp` court est
///   `0xEB`, deux octets lui aussi, et son déplacement se compte depuis
///   le même endroit. L'octet de condition est simplement remplacé.
/// * **saut long** (`0x0F` `0x80`–`0x8F`, six octets) : `jmp` long est
///   `0xE9` suivi de quatre octets — cinq en tout, un de moins. Le
///   sixième est un `nop`, posé **après** le saut, où il ne sera jamais
///   exécuté puisque le saut est désormais toujours pris. Et le
///   déplacement gagne 1 : il se compte depuis la fin de
///   l'instruction, qui vient d'avancer d'un octet. Sans cette
///   correction, le saut arriverait un octet trop tôt.
///
/// Rend `None` sur ce qui n'est pas un saut conditionnel, et sur un
/// déplacement qui déborderait — mieux vaut refuser que d'écrire un
/// saut vers autre chose que ce qui était visé.
pub fn forced_jump(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [opcode, déplacement] if (0x70..=0x7f).contains(opcode) => Some(vec![0xeb, *déplacement]),
        [0x0f, opcode, d0, d1, d2, d3] if (0x80..=0x8f).contains(opcode) => {
            let déplacement = i32::from_le_bytes([*d0, *d1, *d2, *d3]).checked_add(1)?;
            let mut out = vec![0xe9];
            out.extend_from_slice(&déplacement.to_le_bytes());
            // Le compte est bon : cinq octets de saut, un de bourrage.
            out.push(0x90);
            Some(out)
        }
        _ => None,
    }
}

/// Autant de `nop` que d'octets à neutraliser.
///
/// `0x90` est le `nop` d'un octet : on en pose un par octet remplacé, ce
/// qui garde la longueur sans rien déplacer. Un assembleur choisirait
/// des `nop` multi-octets, plus rapides ; ici la lisibilité l'emporte —
/// une suite de `90` se reconnaît d'un coup d'œil dans un vidage, et
/// c'est ce qu'on veut voir en relisant ce qu'on a fait.
pub fn nops(combien: usize) -> Vec<u8> {
    vec![0x90; combien]
}

/// Applique un changement à une image de fichier.
///
/// Rend `None` si la tranche sort du fichier ou si la longueur change :
/// les deux feraient un fichier différent de celui qu'on croit écrire.
pub fn apply(image: &mut [u8], offset: usize, nouveaux: &[u8]) -> Option<Vec<u8>> {
    let fin = offset.checked_add(nouveaux.len())?;
    let place = image.get_mut(offset..fin)?;
    let avant = place.to_vec();
    place.copy_from_slice(nouveaux);
    Some(avant)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les seize conditions courtes vont par paires, et l'inverse est
    /// une involution : l'inverse de l'inverse est l'original.
    ///
    /// C'est ce qui garantit qu'on n'invente rien : si la règle était
    /// fausse pour une condition, l'aller-retour ne retomberait pas sur
    /// ses pieds.
    #[test]
    fn inverting_a_short_jump_twice_gives_it_back() {
        for opcode in 0x70u8..=0x7f {
            let original = vec![opcode, 0x12];
            let inversé = inverted_jump(&original).expect("un saut conditionnel");
            assert_ne!(inversé, original, "{opcode:#x} doit changer");
            assert_eq!(inversé.len(), original.len(), "la longueur ne bouge pas");
            // Le déplacement est intact : on change la condition, pas la
            // cible.
            assert_eq!(inversé[1], 0x12);
            assert_eq!(
                inverted_jump(&inversé).as_deref(),
                Some(original.as_slice()),
                "l'inverse de l'inverse"
            );
        }
    }

    /// Les seize conditions longues aussi.
    #[test]
    fn inverting_a_near_jump_twice_gives_it_back() {
        for opcode in 0x80u8..=0x8f {
            let original = vec![0x0f, opcode, 0x11, 0x22, 0x33, 0x44];
            let inversé = inverted_jump(&original).expect("un saut conditionnel");
            assert_eq!(inversé.len(), 6, "la longueur ne bouge pas");
            assert_eq!(&inversé[2..], &original[2..], "la cible est intacte");
            assert_eq!(
                inverted_jump(&inversé).as_deref(),
                Some(original.as_slice())
            );
        }
    }

    /// Les paires qu'on connaît, nommément.
    ///
    /// L'involution prouve la cohérence ; ces trois-là prouvent qu'on
    /// est bien sur les bonnes conditions, et non sur une paire décalée
    /// d'un cran.
    #[test]
    fn the_pairs_are_the_ones_one_knows() {
        // je (0x74) ↔ jne (0x75)
        assert_eq!(inverted_jump(&[0x74, 0x05]), Some(vec![0x75, 0x05]));
        // jb (0x72) ↔ jae (0x73)
        assert_eq!(inverted_jump(&[0x72, 0x05]), Some(vec![0x73, 0x05]));
        // jle (0x7e) ↔ jg (0x7f)
        assert_eq!(inverted_jump(&[0x7e, 0x05]), Some(vec![0x7f, 0x05]));
        // Et en forme longue : je (0x0f 0x84) ↔ jne (0x0f 0x85)
        assert_eq!(
            inverted_jump(&[0x0f, 0x84, 0, 0, 0, 0]),
            Some(vec![0x0f, 0x85, 0, 0, 0, 0])
        );
    }

    /// Ce qui n'est pas un saut conditionnel est refusé.
    ///
    /// `jmp` n'a pas d'inverse, `jrcxz` non plus, et un `mov` encore
    /// moins. Mieux vaut refuser que d'écrire quelque chose
    /// d'approchant : un octet inventé dans un binaire ne se voit pas,
    /// il se subit à l'exécution.
    #[test]
    fn what_is_not_a_conditional_jump_is_refused() {
        assert_eq!(inverted_jump(&[0xeb, 0x05]), None, "jmp court");
        assert_eq!(inverted_jump(&[0xe9, 0, 0, 0, 0]), None, "jmp long");
        assert_eq!(
            inverted_jump(&[0xe3, 0x05]),
            None,
            "jrcxz n'a pas d'inverse"
        );
        assert_eq!(inverted_jump(&[0x48, 0x89, 0xe5]), None, "mov rbp, rsp");
        assert_eq!(inverted_jump(&[]), None);
        assert_eq!(inverted_jump(&[0x0f]), None, "un préfixe seul");
    }

    /// Neutraliser garde la longueur, toujours.
    #[test]
    fn neutralising_keeps_the_length() {
        assert_eq!(nops(0), Vec::<u8>::new());
        assert_eq!(nops(5), vec![0x90; 5]);
        // Un appel de cinq octets devient cinq `nop` : rien ne bouge
        // autour, et la suite du code garde ses adresses.
        assert_eq!(nops(5).len(), 5);
    }

    /// Écrire dans l'image rend ce qu'il y avait, et refuse ce qui
    /// déborde.
    #[test]
    fn applying_gives_back_what_was_there() {
        let mut image = vec![0x74, 0x05, 0xcc, 0xcc];
        assert_eq!(apply(&mut image, 0, &[0x75, 0x05]), Some(vec![0x74, 0x05]));
        assert_eq!(image, vec![0x75, 0x05, 0xcc, 0xcc]);

        // Et l'on revient en arrière avec ce qu'on a récupéré.
        assert_eq!(apply(&mut image, 0, &[0x74, 0x05]), Some(vec![0x75, 0x05]));
        assert_eq!(image, vec![0x74, 0x05, 0xcc, 0xcc]);

        // Hors du fichier : refusé, et l'image n'a pas bougé.
        assert_eq!(apply(&mut image, 3, &[0, 0]), None);
        assert_eq!(apply(&mut image, 99, &[0]), None);
        assert_eq!(image, vec![0x74, 0x05, 0xcc, 0xcc]);
    }
}

#[cfg(test)]
mod tests_forced {
    use super::*;

    /// Un saut court forcé garde sa longueur et sa cible.
    #[test]
    fn a_short_jump_keeps_its_length_and_its_target() {
        // `je +0x10` à l'adresse 0 : la cible est 0 + 2 + 0x10 = 0x12.
        let forcé = forced_jump(&[0x74, 0x10]).expect("un saut court se force");
        assert_eq!(forcé, vec![0xeb, 0x10], "`jmp` court, même déplacement");
        assert_eq!(forcé.len(), 2, "la longueur ne change pas");
    }

    /// Un saut long forcé garde sa longueur, sa cible, et se complète
    /// d'un `nop` — c'est l'octet de différence entre `0f 8x` et `e9`.
    #[test]
    fn a_near_jump_keeps_its_target_thanks_to_one_added_byte() {
        // `jne +0x40` en six octets à l'adresse 0 : cible 0 + 6 + 0x40.
        let avant = [0x0f, 0x85, 0x40, 0x00, 0x00, 0x00];
        let forcé = forced_jump(&avant).expect("un saut long se force");
        assert_eq!(forcé.len(), avant.len(), "la longueur ne change pas");
        assert_eq!(forcé[0], 0xe9, "`jmp` long");
        assert_eq!(*forcé.last().unwrap(), 0x90, "et un `nop` derrière");

        // La cible est la même, comptée comme le processeur la compte.
        let cible_avant = avant.len() as i64 + i64::from(i32::from_le_bytes([0x40, 0, 0, 0]));
        let d = i32::from_le_bytes([forcé[1], forcé[2], forcé[3], forcé[4]]);
        let cible_après = 5_i64 + i64::from(d);
        assert_eq!(
            cible_avant, cible_après,
            "le saut forcé doit arriver exactement où l'autre arrivait"
        );
    }

    /// Ce qui n'est pas un saut conditionnel est refusé, et `jrcxz`
    /// avec — il n'a pas d'équivalent inconditionnel d'une instruction.
    #[test]
    fn what_is_not_a_conditional_jump_is_refused() {
        assert_eq!(forced_jump(&[0x90]), None, "un `nop`");
        assert_eq!(
            forced_jump(&[0xeb, 0x10]),
            None,
            "un `jmp` déjà inconditionnel"
        );
        assert_eq!(forced_jump(&[0xe3, 0x10]), None, "`jrcxz`");
        assert_eq!(forced_jump(&[]), None, "rien du tout");
        // Un saut court à qui il manque son déplacement : refusé plutôt
        // que complété au hasard.
        assert_eq!(forced_jump(&[0x74]), None);
        // Un déplacement au maximum : ajouter 1 déborderait.
        assert_eq!(forced_jump(&[0x0f, 0x84, 0xff, 0xff, 0xff, 0x7f]), None);
    }
}
