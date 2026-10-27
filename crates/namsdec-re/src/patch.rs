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
    /// Ce qu'on a mis à la place.
    ///
    /// De la même longueur que `before` pour tout ce que namsdec pose
    /// lui-même — inverser un saut, neutraliser, détourner —, et c'est
    /// la règle qui rend ces gestes sûrs. Une ÉCRITURE À LA MAIN peut
    /// en changer la longueur : voir [`splice`], qui dit ce que cela
    /// coûte.
    pub after: Vec<u8>,
    /// L'adresse virtuelle correspondante, pour le dire à l'écran.
    pub addr: u64,
    /// Les correctifs qui ne valent QUE posés ensemble partagent ce
    /// numéro.
    ///
    /// Un détour en fait deux : le saut à l'endroit visé, et le corps
    /// dans la place libre. Défaire le corps seul laisserait un `jmp`
    /// vers du bourrage — le programme sauterait dans des zéros. Les
    /// deux portent donc le même numéro, et l'interface défait le
    /// groupe entier plutôt qu'une moitié.
    ///
    /// `None` pour un correctif qui se tient seul, ce qui est le cas de
    /// tous ceux d'avant le détour : inverser un saut, neutraliser,
    /// écrire des octets à la main.
    pub group: Option<u64>,
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

/// Remplace `combien` octets à `offset` par `nouveaux`, et rend ceux
/// qu'on a retirés.
///
/// C'est la forme générale dont [`apply`] est le cas sûr. Elle accepte
/// trois choses que l'autre refuse, et il faut dire ce qu'elles
/// coûtent :
///
/// - `combien == nouveaux.len()` — un remplacement, rien ne bouge.
///   C'est [`apply`], et c'est le seul cas où le fichier reste ce qu'il
///   était ailleurs qu'ici ;
/// - `combien == 0` — une **insertion**. Le fichier grandit, et TOUT CE
///   QUI SUIT SE DÉCALE : chaque position devient fausse, et dans un
///   ELF, chaque en-tête qui désigne un segment ou une section désigne
///   désormais autre chose. L'appelant doit s'attendre à ce que le
///   fichier ne se relise plus — c'est le cas ordinaire, et non
///   l'accident ;
/// - `nouveaux.is_empty()` — une **suppression**, avec le même effet en
///   sens inverse.
///
/// Elle ne juge pas : elle fait ce qu'on demande et rend de quoi le
/// défaire. C'est à l'interface de prévenir, et à `load_elf` de refuser
/// ce qu'il ne sait plus lire.
///
/// `None` quand la tranche demandée sort du fichier — on ne devine pas
/// ce qu'il y aurait au-delà.
pub fn splice(
    image: &mut Vec<u8>,
    offset: usize,
    combien: usize,
    nouveaux: &[u8],
) -> Option<Vec<u8>> {
    let fin = offset.checked_add(combien)?;
    if fin > image.len() {
        return None;
    }
    let retirés = image[offset..fin].to_vec();
    image.splice(offset..fin, nouveaux.iter().copied());
    Some(retirés)
}

/// Le correctif déjà posé qui occupe ces octets, s'il y en a un.
///
/// Deux correctifs qui se chevauchent ne se défont pas : l'annulation
/// du premier réécrit des octets que le second avait posés, et le
/// fichier obtenu dépend de l'ordre dans lequel on les retire. Cette
/// fonction est donc la garde de la réversibilité, et c'est pour cela
/// qu'elle est publique : l'interface s'en sert une seconde fois au
/// moment de POSER, parce qu'un plan calculé reste affiché et qu'un
/// autre correctif peut être venu entre-temps.
///
/// L'intervalle est celui des octets ÉCRITS — `offset` à
/// `offset + after.len()` —, et non celui des octets remplacés : une
/// écriture à la main peut changer la longueur (voir [`splice`]), et
/// c'est ce qui occupe la place aujourd'hui qui compte.
pub fn chevauche(posés: &[Patch], offset: usize, combien: usize) -> Option<&Patch> {
    let fin = offset.saturating_add(combien);
    posés
        .iter()
        .find(|p| offset < p.offset + p.after.len() && p.offset < fin)
}

// ---------------------------------------------------------------------
// Le détour : sortir du cadre de la longueur exacte, sans rien décaler.
// ---------------------------------------------------------------------
//
// Tout ce qui précède garde la longueur exacte, et c'est ce qui le rend
// sûr. Mais cette règle interdit la moitié de ce qu'on veut faire : on
// ne peut pas AJOUTER quelque chose. Trois octets de plus et chaque
// adresse du fichier devient fausse.
//
// Le détour contourne l'obstacle au lieu de l'affronter. L'idée est
// vieille comme les correctifs binaires, et elle tient en trois temps :
//
//   1. à l'endroit visé, on écrit un `jmp` vers de la place libre —
//      celle que `slack` a trouvée, le bourrage que l'éditeur de liens
//      a laissé pour aligner ;
//   2. dans cette place, on écrit le code qu'on voulait ajouter, puis
//      les instructions qu'on a dû recouvrir pour poser le `jmp` ;
//   3. on termine par un `jmp` de retour, juste après l'endroit visé.
//
// Le programme passe désormais par un détour, exécute ce qu'on a
// ajouté, retrouve ce qu'il faisait, et continue. Le fichier n'a pas
// changé de taille : on a seulement rempli des trous.
//
// C'est là que s'arrête la facilité, et les refus ci-dessous valent
// autant que le geste lui-même.

/// La longueur d'un `jmp rel32` : l'opcode `0xE9`, puis quatre octets de
/// déplacement.
///
/// C'est le plus petit saut qui atteigne n'importe où dans un binaire
/// ordinaire, et donc la place minimale qu'il faut recouvrir à l'endroit
/// visé. Un `jmp rel8` ne tiendrait qu'en deux octets, mais ne porte
/// qu'à 127 octets — jamais jusqu'à la fin d'une section.
pub const JMP_REL32: usize = 5;

/// Un `jmp rel32` de `depuis` vers `vers`, en adresses virtuelles.
///
/// Le déplacement se compte **depuis la fin de l'instruction**, pas
/// depuis son début : le processeur a déjà avancé `rip` de cinq octets
/// quand il l'applique. C'est l'erreur la plus courante à la main, et
/// elle donne un saut qui atterrit cinq octets trop loin.
///
/// Rend `None` quand la distance ne tient pas dans un entier signé de
/// 32 bits — au-delà de deux gigaoctets, ce saut n'existe pas, et il
/// faudrait passer par un registre. Mieux vaut refuser que tronquer.
pub fn jmp_rel32(depuis: u64, vers: u64) -> Option<Vec<u8>> {
    let fin = depuis.checked_add(JMP_REL32 as u64)?;
    let écart = (vers as i64).checked_sub(fin as i64)?;
    let écart = i32::try_from(écart).ok()?;

    let mut octets = Vec::with_capacity(JMP_REL32);
    octets.push(0xE9);
    octets.extend_from_slice(&écart.to_le_bytes());
    Some(octets)
}

/// Pourquoi un détour n'a pas pu être posé.
///
/// Chaque variante est un refus, pas un avertissement : le détour n'est
/// pas écrit. Un correctif qu'on pose « presque » laisse un binaire qui
/// plante plus tard et ailleurs, et c'est exactement ce qu'un outil
/// d'apprentissage ne doit pas faire — on n'apprend rien d'un plantage
/// dont la cause est à trois sections de là.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetourRefusé {
    /// La place libre choisie n'est pas dans un segment exécutable.
    ///
    /// Un trou de mille octets en plein `.rodata` ne sert à rien : le
    /// `jmp` y arriverait, et le processeur refuserait d'exécuter la
    /// page. C'est le premier contrôle parce que c'est le piège le plus
    /// tentant — `slack::find` trie par taille, et les plus grands trous
    /// d'un binaire sont souvent dans les données.
    PlaceNonExécutable,
    /// La place libre ne suffit pas.
    PlaceTropPetite { besoin: u64, offert: u64 },
    /// L'endroit visé, ou la place libre, sort du fichier.
    HorsFichier,
    /// Impossible de décoder assez d'instructions entières à l'endroit
    /// visé pour y loger les cinq octets du `jmp`.
    SiteIndécodable { à: u64 },
    /// La distance dépasse ce qu'un `jmp rel32` peut franchir.
    TropLoin,
    /// Une instruction à déplacer dépend de sa position.
    ///
    /// C'est le refus qui compte vraiment, et le moins visible. Les
    /// instructions recouvertes par le `jmp` doivent être réécrites dans
    /// la place libre — donc à une AUTRE adresse. Celles qui s'adressent
    /// relativement à `rip` (`mov rax, [rip+0x2e4f]`, très courant en
    /// code compilé moderne) ou qui sautent relativement (`call`, `jne`)
    /// liraient alors la mauvaise donnée, ou sauteraient au mauvais
    /// endroit, sans rien signaler.
    ///
    /// Les réécrire correctement s'appelle **relocaliser**, et c'est ce
    /// que [`relocate`] fait désormais pour les deux formes courantes :
    /// un déplacement de quatre octets relatif à `rip`, et un saut ou
    /// un appel relatif de quatre octets. Ce refus ne reste donc que
    /// pour ce qui ne se relocalise pas en gardant la longueur — voir
    /// [`Self::PortéeTropCourte`] et [`Self::ChampIntrouvable`].
    CodeDéplacéRelatif { à: u64, mnémonique: String },
    /// Le champ à réécrire ne porte pas jusqu'à la cible depuis la
    /// place libre.
    ///
    /// Un saut court (`jmp rel8`, deux octets) atteint 127 octets ; la
    /// place libre est presque toujours plus loin. L'élargir en
    /// `jmp rel32` demanderait trois octets de plus, donc changerait la
    /// longueur — précisément ce que le détour ne peut pas faire dans
    /// la zone qu'il recouvre. C'est la limite honnête de la méthode, et
    /// non un défaut d'implémentation.
    PortéeTropCourte {
        à: u64,
        mnémonique: String,
        champ: usize,
    },
    /// L'instruction dépend de sa position, mais le décodeur ne dit pas
    /// où se trouve le champ à réécrire.
    ///
    /// Ne devrait pas arriver sur du code valide. On refuse plutôt que
    /// d'écrire au hasard dans une instruction.
    ChampIntrouvable { à: u64, mnémonique: String },
    /// Les octets que ce détour écrirait sont déjà ceux d'un correctif
    /// posé.
    ///
    /// C'est le refus qui protège le travail DÉJÀ FAIT, et il manquait.
    /// Un détour écrit à deux endroits : cinq octets à l'endroit visé,
    /// et son corps dans la place libre. Si un correctif occupe l'un ou
    /// l'autre — un autre détour, un saut inversé, une écriture faite à
    /// la main depuis le vidage —, le second passe PAR-DESSUS le
    /// premier, et deux dégâts s'ensuivent, tous deux silencieux.
    ///
    /// Le premier détour perd son saut : ses cinq octets réécrits, son
    /// corps devient inatteignable, et le programme fait de nouveau ce
    /// qu'il faisait avant qu'on le corrige — sauf si le nouveau saut
    /// tombe au milieu de l'ancien, et alors le déplacement est à
    /// moitié réécrit et le programme part dans le vide. C'est
    /// exactement ce qu'un crochet Frida posé sur `main` a fait à un
    /// détour de namsdec le 15 octobre 2026 : deux détours au même
    /// endroit ne cohabitent pas, et cela vaut des deux côtés.
    ///
    /// Le second est pour DÉFAIRE. Les `before` des deux correctifs se
    /// chevauchent alors, et l'annulation du premier réécrit des octets
    /// que le second avait posés : l'ordre dans lequel on les défait
    /// change le fichier obtenu. Un correctif qui ne se défait pas de
    /// façon unique n'est plus réversible, et la réversibilité est la
    /// promesse de toute cette fenêtre.
    SurUnCorrectif {
        /// L'adresse virtuelle de ce que le détour voulait écrire là.
        à: u64,
        /// Où, dans le fichier, les deux écritures se croisent.
        offset: usize,
        /// L'adresse du correctif déjà posé. Zéro pour une écriture
        /// faite depuis le vidage, où l'on désigne une position et non
        /// une instruction — d'où `offset`, qui vaut toujours.
        correctif: u64,
    },
}

impl DetourRefusé {
    /// Ce qu'on en montre à l'écran.
    pub fn message(&self, fr: bool) -> String {
        match self {
            Self::PlaceNonExécutable if fr => {
                "cette place libre n'est pas exécutable : le programme ne \
                 pourrait pas y courir"
                    .to_string()
            }
            Self::PlaceNonExécutable => {
                "this slack is not executable: the program could not run there".to_string()
            }
            Self::PlaceTropPetite { besoin, offert } if fr => {
                format!("il faut {besoin} octets de place libre, celle-ci en offre {offert}")
            }
            Self::PlaceTropPetite { besoin, offert } => {
                format!("{besoin} bytes of slack needed, this one offers {offert}")
            }
            Self::HorsFichier if fr => "cette adresse sort du fichier".to_string(),
            Self::HorsFichier => "this address lies outside the file".to_string(),
            Self::SiteIndécodable { à } if fr => format!(
                "impossible de décoder {JMP_REL32} octets d'instructions entières en {à:#x}"
            ),
            Self::SiteIndécodable { à } => {
                format!("cannot decode {JMP_REL32} bytes of whole instructions at {à:#x}")
            }
            Self::TropLoin if fr => {
                "la distance dépasse les deux gigaoctets d'un jmp rel32".to_string()
            }
            Self::TropLoin => "the distance exceeds the 2 GB reach of a jmp rel32".to_string(),
            Self::CodeDéplacéRelatif { à, mnémonique } if fr => format!(
                "`{mnémonique}` en {à:#x} dépend de sa position : la déplacer \
                 la ferait lire ou sauter ailleurs. namsdec ne relocalise pas \
                 encore — choisis un autre endroit, un peu plus haut ou plus bas"
            ),
            Self::CodeDéplacéRelatif { à, mnémonique } => format!(
                "`{mnémonique}` at {à:#x} is position-dependent in a form \
                 namsdec cannot relocate — pick another site, slightly above \
                 or below"
            ),
            Self::PortéeTropCourte {
                à,
                mnémonique,
                champ,
            } if fr => format!(
                "`{mnémonique}` en {à:#x} vise sa cible sur {champ} octet(s) : \
                 trop court pour atteindre la place libre. L'élargir changerait \
                 sa longueur, ce que le détour ne peut pas faire ici — choisis \
                 un autre endroit"
            ),
            Self::PortéeTropCourte {
                à,
                mnémonique,
                champ,
            } => format!(
                "`{mnémonique}` at {à:#x} addresses its target in {champ} byte(s): \
                 too short to reach the slack. Widening it would change its \
                 length, which the detour cannot do here — pick another site"
            ),
            Self::ChampIntrouvable { à, mnémonique } if fr => format!(
                "`{mnémonique}` en {à:#x} dépend de sa position, mais le décodeur \
                 ne dit pas où est le champ à réécrire — on ne l'écrit pas au hasard"
            ),
            Self::ChampIntrouvable { à, mnémonique } => format!(
                "`{mnémonique}` at {à:#x} is position-dependent, but the decoder \
                 does not say where the field to rewrite is — it will not be guessed"
            ),
            Self::SurUnCorrectif {
                à,
                offset,
                correctif,
            } if fr => format!(
                "ce détour écrirait en {à:#x} (position {offset:#x}) sur les octets \
                 d'un correctif déjà posé en {correctif:#x} : le premier perdrait \
                 son effet, et les deux ne se défont plus dans un ordre unique. \
                 Défais-le par la fenêtre Correctifs, ou vise une autre instruction"
            ),
            Self::SurUnCorrectif {
                à,
                offset,
                correctif,
            } => format!(
                "this detour would write at {à:#x} (offset {offset:#x}) over bytes \
                 already changed by a patch at {correctif:#x}: the first would lose \
                 its effect, and neither could be undone in a single order. Undo it \
                 from the Patches window, or aim at another instruction"
            ),
        }
    }
}

/// Réécrit une instruction pour qu'elle fasse la même chose depuis une
/// autre adresse.
///
/// C'est tout le sujet du déplacement. Une instruction comme
/// `lea rsi, [rip+0x2b8a]` ne dit pas « va chercher à 0x40300c » : elle
/// dit « va chercher 0x2b8a octets après la fin de cette
/// instruction-ci ». Déplacée telle quelle, elle compte à partir d'un
/// autre endroit, et lit autre chose. On ne touche donc pas à ce qu'elle
/// fait — on recalcule son écart pour que la **somme** retombe sur la
/// même cible.
///
/// Deux champs se relocalisent ainsi, et aucun encodeur n'est nécessaire
/// — le décodeur dit où ils commencent et combien d'octets ils font, et
/// l'instruction garde sa longueur :
///
/// - le **déplacement** d'un accès mémoire relatif à `rip` ;
/// - l'**immédiat** d'un saut ou d'un appel relatif.
///
/// Un seul des deux cas peut se présenter à la fois : un accès relatif
/// à `rip` n'est jamais un saut relatif.
///
/// Rend l'instruction inchangée quand elle ne dépend pas de sa
/// position : c'est le cas courant, et il n'y a rien à faire.
pub fn relocate(
    octets: &[u8],
    instr: &iced_x86::Instruction,
    champs: &iced_x86::ConstantOffsets,
    nouvelle_ip: u64,
) -> Result<Vec<u8>, DetourRefusé> {
    use iced_x86::OpKind;

    let mnémonique = || format!("{:?}", instr.mnemonic()).to_lowercase();
    let à = instr.ip();

    // La fin de l'instruction, à sa nouvelle place : c'est de là que
    // tout écart relatif se compte.
    let fin = nouvelle_ip
        .checked_add(instr.len() as u64)
        .ok_or(DetourRefusé::HorsFichier)?;

    // Le champ à réécrire, et la cible absolue à retrouver. L'ordre
    // compte : on ne touche l'immédiat que s'il porte une CIBLE. Celui
    // de `mov dword [rip+0x10], 1` est une donnée, et le réécrire
    // changerait ce que le programme calcule.
    let branche = (0..instr.op_count()).any(|i| {
        matches!(
            instr.op_kind(i),
            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64
        )
    });

    let (début, taille, cible) = if instr.is_ip_rel_memory_operand() {
        (
            champs.displacement_offset(),
            champs.displacement_size(),
            instr.ip_rel_memory_address(),
        )
    } else if branche {
        (
            champs.immediate_offset(),
            champs.immediate_size(),
            instr.near_branch_target(),
        )
    } else {
        // Rien de relatif : elle se déplace telle quelle.
        return Ok(octets.to_vec());
    };

    if taille == 0 || début + taille > octets.len() {
        return Err(DetourRefusé::ChampIntrouvable {
            à,
            mnémonique: mnémonique(),
        });
    }

    let écart = (cible as i64)
        .checked_sub(fin as i64)
        .ok_or(DetourRefusé::HorsFichier)?;

    // L'écart doit tenir dans le champ TEL QU'IL EST. L'élargir
    // changerait la longueur de l'instruction, et le détour ne peut pas
    // se le permettre dans la zone qu'il recouvre.
    let trop_court = DetourRefusé::PortéeTropCourte {
        à,
        mnémonique: mnémonique(),
        champ: taille,
    };
    let champ: Vec<u8> = match taille {
        1 => i8::try_from(écart)
            .map_err(|_| trop_court)?
            .to_le_bytes()
            .to_vec(),
        2 => i16::try_from(écart)
            .map_err(|_| trop_court)?
            .to_le_bytes()
            .to_vec(),
        4 => i32::try_from(écart)
            .map_err(|_| trop_court)?
            .to_le_bytes()
            .to_vec(),
        // Aucun déplacement ni immédiat relatif ne fait huit octets sur
        // x86-64 : `jmp rel64` n'existe pas, et un déplacement est au
        // plus de quatre octets. Un champ d'une autre taille veut dire
        // qu'on a mal lu.
        _ => {
            return Err(DetourRefusé::ChampIntrouvable {
                à,
                mnémonique: mnémonique(),
            });
        }
    };

    let mut réécrite = octets.to_vec();
    réécrite[début..début + taille].copy_from_slice(&champ);
    Ok(réécrite)
}

/// Un détour prêt à poser : deux changements, et rien d'autre.
///
/// Les deux sont des [`Patch`] ordinaires, avec leur `before` — donc le
/// détour se défait comme n'importe quel correctif, par la fenêtre qui
/// les liste. C'était la condition pour que cette fonctionnalité passe
/// « par là plutôt que par un chemin parallèle » : un détour n'est pas
/// une catégorie à part, c'est deux écritures d'octets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detour {
    /// À l'endroit visé : le `jmp` vers la place libre, complété de
    /// `nop` si les instructions recouvertes faisaient plus de cinq
    /// octets.
    pub aller: Patch,
    /// Dans la place libre : le code ajouté, les instructions
    /// déplacées, puis le `jmp` de retour.
    pub corps: Patch,
    /// Combien d'octets ont été recouverts à l'endroit visé — toujours
    /// un nombre entier d'instructions, et au moins [`JMP_REL32`].
    pub recouvert: usize,
    /// Les instructions recouvertes, telles qu'elles sont réécrites dans
    /// la place libre. Les montrer permet de vérifier d'un coup d'œil ce
    /// que le détour a déménagé.
    pub déplacé: Vec<u8>,
}

/// Planifie un détour, sans rien écrire.
///
/// Rien n'est modifié : la fonction rend les deux changements à poser,
/// ou dit pourquoi elle refuse. C'est voulu — on veut pouvoir montrer le
/// plan avant de l'appliquer, et un plan qu'on calcule en écrivant ne
/// se montre pas.
///
/// `ajout` est le code à insérer, déjà assemblé. Il doit se terminer par
/// un chemin qui tombe en séquence : le retour est ajouté après lui, et
/// un `ret` au milieu de `ajout` sauterait le reste du détour — ce qui
/// se fait exprès parfois, mais jamais par accident.
///
/// `posés` est la liste des correctifs DÉJÀ en place. Le plan se calcule
/// sur l'image telle qu'elle est — donc déjà corrigée —, mais les octets
/// ne disent pas d'où ils viennent : un `jmp` posé par un premier détour
/// se décode comme n'importe quel `jmp`, et rien, dans l'image seule, ne
/// distingue le code d'origine du travail qu'on y a fait. C'est cette
/// liste qui le dit, et sans elle le second détour écrase le premier en
/// silence. Passer `&[]` revient à affirmer qu'aucun correctif n'est
/// posé, ce qui est vrai d'un fichier qu'on vient d'ouvrir.
pub fn plan_detour(
    image: &[u8],
    bitness: u32,
    site_offset: usize,
    site_vaddr: u64,
    place: &crate::slack::Slack,
    ajout: &[u8],
    posés: &[Patch],
) -> Result<Detour, DetourRefusé> {
    use iced_x86::{Decoder, DecoderOptions};

    if !place.executable {
        return Err(DetourRefusé::PlaceNonExécutable);
    }

    // L'ENDROIT VISÉ, AVANT DE LE DÉCODER. Le détour y recouvrira au
    // moins les cinq octets du `jmp` ; si un correctif en occupe un
    // seul, c'est déjà perdu, et il vaut mieux le dire ici qu'après
    // avoir décodé. Sans quoi un site déjà détourné rend le refus de
    // l'instruction qui s'y trouve — « `jmp` dépend de sa position » —,
    // qui est vrai mais ne nomme pas la cause.
    if let Some(p) = chevauche(posés, site_offset, JMP_REL32) {
        return Err(DetourRefusé::SurUnCorrectif {
            à: site_vaddr,
            offset: site_offset,
            correctif: p.addr,
        });
    }

    // Les instructions à recouvrir : on décode à partir de l'endroit
    // visé et on en prend des ENTIÈRES jusqu'à couvrir les cinq octets
    // du `jmp`. Jamais un nombre d'octets fixe : couper une instruction
    // en deux laisserait sa queue dans le fichier, et le processeur la
    // décoderait comme autre chose au retour du détour.
    let reste = image.get(site_offset..).ok_or(DetourRefusé::HorsFichier)?;
    let mut décodeur = Decoder::with_ip(bitness, reste, site_vaddr, DecoderOptions::NONE);
    let mut recouvert = 0usize;
    let mut à_déplacer = Vec::new();
    while recouvert < JMP_REL32 {
        if !décodeur.can_decode() {
            return Err(DetourRefusé::SiteIndécodable { à: site_vaddr });
        }
        let instr = décodeur.decode();
        if instr.is_invalid() {
            return Err(DetourRefusé::SiteIndécodable { à: site_vaddr });
        }
        // La position des champs relatifs, telle que le décodeur l'a
        // vue. À prendre MAINTENANT : elle porte sur la dernière
        // instruction décodée, et la prochaine l'effacera.
        let champs = décodeur.get_constant_offsets(&instr);
        let octets = reste
            .get(recouvert..recouvert + instr.len())
            .ok_or(DetourRefusé::HorsFichier)?
            .to_vec();
        à_déplacer.push((instr, champs, octets));
        recouvert += instr.len();
    }

    // Les instructions, réécrites pour leur nouvelle adresse.
    //
    // Leur place dans le corps du détour est connue d'avance : l'ajout
    // vient d'abord, puis elles, dans l'ordre. Et comme relocaliser ne
    // change pas la longueur d'une instruction, cette place ne dépend
    // pas du résultat — on peut donc calculer la nouvelle adresse de
    // chacune avant de la réécrire.
    let mut déplacé = Vec::with_capacity(recouvert);
    for (instr, champs, octets) in &à_déplacer {
        let nouvelle_ip = place
            .vaddr
            .checked_add((ajout.len() + déplacé.len()) as u64)
            .ok_or(DetourRefusé::HorsFichier)?;
        let réécrite = relocate(octets, instr, champs, nouvelle_ip)?;
        debug_assert_eq!(
            réécrite.len(),
            octets.len(),
            "relocaliser ne change pas la longueur"
        );
        déplacé.extend_from_slice(&réécrite);
    }

    // La place qu'il faut : le code ajouté, les instructions déplacées,
    // et le saut de retour.
    let besoin = (ajout.len() + déplacé.len() + JMP_REL32) as u64;
    if place.len < besoin {
        return Err(DetourRefusé::PlaceTropPetite {
            besoin,
            offert: place.len,
        });
    }

    // L'aller : le `jmp`, puis des `nop` jusqu'à la fin de ce qu'on a
    // recouvert. Les `nop` ne s'exécutent jamais — le `jmp` part avant
    // — mais ils comptent : sans eux, la queue de la dernière
    // instruction recouverte resterait lisible, et un désassemblage du
    // fichier corrigé montrerait une instruction fantôme à cet endroit.
    let mut aller = jmp_rel32(site_vaddr, place.vaddr).ok_or(DetourRefusé::TropLoin)?;
    aller.extend_from_slice(&nops(recouvert - JMP_REL32));

    // Le corps : l'ajout, le déplacé, le retour. Le retour part de la
    // fin du déplacé et vise l'instruction qui suit ce qu'on a recouvert.
    let retour_depuis = place
        .vaddr
        .checked_add((ajout.len() + déplacé.len()) as u64)
        .ok_or(DetourRefusé::HorsFichier)?;
    let retour_vers = site_vaddr
        .checked_add(recouvert as u64)
        .ok_or(DetourRefusé::HorsFichier)?;
    let retour = jmp_rel32(retour_depuis, retour_vers).ok_or(DetourRefusé::TropLoin)?;

    let mut corps = Vec::with_capacity(besoin as usize);
    corps.extend_from_slice(ajout);
    corps.extend_from_slice(&déplacé);
    corps.extend_from_slice(&retour);

    // Les `before` des deux changements, pour pouvoir les défaire.
    let place_offset = usize::try_from(place.offset).map_err(|_| DetourRefusé::HorsFichier)?;
    let avant_site = image
        .get(site_offset..site_offset + aller.len())
        .ok_or(DetourRefusé::HorsFichier)?
        .to_vec();
    let avant_place = image
        .get(place_offset..place_offset + corps.len())
        .ok_or(DetourRefusé::HorsFichier)?
        .to_vec();

    // LES DEUX ÉCRITURES EN ENTIER, cette fois. La garde du début ne
    // connaissait que les cinq octets du `jmp` ; on sait maintenant ce
    // que le détour recouvre vraiment — jusqu'à quinze octets si la
    // dernière instruction est longue — et où son corps atterrit. Le
    // corps compte autant : `slack::find` reconnaît le bourrage à
    // lui-même, et il ne reste donc que la queue d'un trou déjà employé
    // ; mais un plan reste affiché après son calcul, et la place qu'il
    // visait peut avoir été prise depuis.
    for (quoi, offset, combien) in [
        (site_vaddr, site_offset, aller.len()),
        (place.vaddr, place_offset, corps.len()),
    ] {
        if let Some(p) = chevauche(posés, offset, combien) {
            return Err(DetourRefusé::SurUnCorrectif {
                à: quoi,
                offset,
                correctif: p.addr,
            });
        }
    }

    // Le numéro du groupe se déduit du site : le même plan rend le même
    // numéro, et deux détours posés à des endroits différents ne se
    // mélangent pas. Pas de compteur à tenir, donc rien à remettre à
    // zéro ni à sauvegarder.
    let groupe = Some(site_vaddr);

    Ok(Detour {
        aller: Patch {
            offset: site_offset,
            before: avant_site,
            after: aller,
            addr: site_vaddr,
            group: groupe,
        },
        corps: Patch {
            offset: place_offset,
            before: avant_place,
            after: corps,
            addr: place.vaddr,
            group: groupe,
        },
        recouvert,
        déplacé,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `splice` couvre les trois cas, et le troisième est celui qui
    /// manquait : ajouter.
    #[test]
    fn splice_replaces_inserts_and_deletes() {
        // Remplacer : rien ne bouge ailleurs, et c'est `apply`.
        let mut image = vec![1u8, 2, 3, 4];
        assert_eq!(splice(&mut image, 1, 2, &[9, 9]), Some(vec![2, 3]));
        assert_eq!(image, vec![1, 9, 9, 4]);

        // Insérer : le fichier grandit, et tout ce qui suit se décale.
        let mut image = vec![1u8, 2, 3, 4];
        assert_eq!(splice(&mut image, 2, 0, &[7, 7, 7]), Some(Vec::new()));
        assert_eq!(image, vec![1, 2, 7, 7, 7, 3, 4]);

        // Supprimer : le même effet en sens inverse.
        let mut image = vec![1u8, 2, 3, 4];
        assert_eq!(splice(&mut image, 1, 2, &[]), Some(vec![2, 3]));
        assert_eq!(image, vec![1, 4]);

        // Au-delà du fichier, on ne devine pas : rien n'est touché.
        let mut image = vec![1u8, 2];
        assert_eq!(splice(&mut image, 1, 5, &[0]), None);
        assert_eq!(image, vec![1, 2]);
    }

    /// Ce qu'une insertion a fait se défait exactement.
    ///
    /// C'est la condition de tout le reste : un correctif qu'on ne peut
    /// pas défaire n'est pas un outil d'analyse, c'est une décision
    /// définitive. Défaire une insertion demande de RETIRER ce qu'on a
    /// écrit — `after.len()` octets — et non d'en réécrire autant
    /// qu'on en avait pris, ce qui aurait écrasé les voisins.
    #[test]
    fn an_insertion_is_undone_exactly() {
        let original = vec![0x7fu8, 0x45, 0x4c, 0x46, 0xaa, 0xbb];
        for (offset, retirés, écrits) in [
            (2usize, 0usize, vec![0x90u8, 0x90]),
            (4, 2, vec![0x11]),
            (0, 1, vec![0x22, 0x33, 0x44]),
            (6, 0, vec![0xff]),
        ] {
            let mut image = original.clone();
            let avant = splice(&mut image, offset, retirés, &écrits).expect("la tranche existe");
            assert_ne!(image, original, "le correctif doit changer quelque chose");
            // Défaire : retirer ce qu'on a écrit, remettre ce qu'on
            // avait pris.
            splice(&mut image, offset, écrits.len(), &avant).expect("la tranche existe encore");
            assert_eq!(image, original, "défait à {offset:#x}");
        }
    }

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

#[cfg(test)]
mod tests_detour {
    use super::*;
    use crate::slack::Slack;

    /// Une place libre exécutable, posée où on veut.
    fn place(vaddr: u64, offset: u64, len: u64) -> Slack {
        Slack {
            vaddr,
            offset,
            len,
            filler: 0x00,
            executable: true,
            section: Some(".text".to_string()),
        }
    }

    /// Le déplacement se compte depuis la FIN de l'instruction.
    ///
    /// Un saut de 0x1000 vers 0x1005 ne vaut pas 5 mais 0 : les cinq
    /// octets du `jmp` sont déjà consommés quand le processeur applique
    /// le déplacement. C'est l'erreur qui donne un atterrissage cinq
    /// octets trop loin, et elle ne se voit pas à la relecture.
    #[test]
    fn a_jump_counts_from_its_own_end() {
        assert_eq!(jmp_rel32(0x1000, 0x1005).unwrap(), vec![0xE9, 0, 0, 0, 0]);
        // En avant de 0x10 octets après la fin.
        assert_eq!(
            jmp_rel32(0x1000, 0x1015).unwrap(),
            vec![0xE9, 0x10, 0, 0, 0]
        );
        // En arrière : le déplacement est négatif, en complément à deux.
        assert_eq!(
            jmp_rel32(0x1005, 0x1000).unwrap(),
            vec![0xE9, 0xF6, 0xFF, 0xFF, 0xFF]
        );
    }

    /// Au-delà de deux gigaoctets, ce saut n'existe pas — on refuse au
    /// lieu de tronquer le déplacement.
    #[test]
    fn a_jump_too_far_is_refused_rather_than_truncated() {
        assert_eq!(jmp_rel32(0, 0x8000_0005), None);
        assert_eq!(jmp_rel32(0x8000_0000, 0), None);
        // Juste à la limite, il tient encore.
        assert!(jmp_rel32(0, 0x7FFF_FFFF + JMP_REL32 as u64).is_some());
    }

    /// Le détour complet, posé et relu : l'aller saute dans la place, le
    /// corps rend la main juste après ce qu'on a recouvert.
    ///
    /// C'est le contrôle de bout en bout. On vérifie les deux
    /// déplacements par le calcul inverse, parce qu'un détour dont un
    /// seul des deux sauts est juste s'exécute une fois et part dans le
    /// vide ensuite.
    #[test]
    fn a_detour_leaves_and_comes_back_to_the_right_place() {
        // `mov eax, 1` (5 octets) puis `ret`, à l'adresse 0x1000.
        let mut image = vec![0u8; 0x200];
        image[0x00..0x05].copy_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
        image[0x05] = 0xC3;

        let libre = place(0x1100, 0x100, 64);
        // Le code ajouté : `nop` deux fois, de quoi se reconnaître.
        let detour = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[0x90, 0x90], &[])
            .expect("ce détour est posable");

        // Cinq octets recouverts : `mov eax, 1` en fait exactement cinq,
        // donc aucun `nop` de complément.
        assert_eq!(detour.recouvert, 5);
        assert_eq!(detour.déplacé, vec![0xB8, 0x01, 0x00, 0x00, 0x00]);
        assert_eq!(detour.aller.after.len(), 5, "le jmp remplit la place");

        // L'aller vise bien la place libre.
        let écart = i32::from_le_bytes(detour.aller.after[1..5].try_into().unwrap());
        assert_eq!(
            (0x1000i64 + JMP_REL32 as i64 + écart as i64) as u64,
            0x1100,
            "l'aller atterrit dans la place libre"
        );

        // Le corps : ajout, déplacé, retour.
        assert_eq!(&detour.corps.after[..2], &[0x90, 0x90], "le code ajouté");
        assert_eq!(
            &detour.corps.after[2..7],
            &[0xB8, 0x01, 0x00, 0x00, 0x00],
            "puis ce qu'on a recouvert"
        );
        assert_eq!(detour.corps.after[7], 0xE9, "puis le retour");

        // Le retour tombe sur le `ret`, pas au milieu du `mov`.
        let retour_depuis = 0x1100u64 + 7;
        let écart = i32::from_le_bytes(detour.corps.after[8..12].try_into().unwrap());
        assert_eq!(
            (retour_depuis as i64 + JMP_REL32 as i64 + écart as i64) as u64,
            0x1005,
            "le retour reprend juste après ce qui a été recouvert"
        );
    }

    /// Les deux changements se défont : c'est la condition pour que le
    /// détour passe par les correctifs ordinaires.
    #[test]
    fn a_detour_can_be_undone_byte_for_byte() {
        let mut image = vec![0u8; 0x200];
        image[0x00..0x05].copy_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
        let original = image.clone();

        let libre = place(0x1100, 0x100, 64);
        let detour = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[0x90], &[]).unwrap();

        apply(&mut image, detour.aller.offset, &detour.aller.after).unwrap();
        apply(&mut image, detour.corps.offset, &detour.corps.after).unwrap();
        assert_ne!(image, original, "le détour a bien changé le fichier");

        apply(&mut image, detour.corps.offset, &detour.corps.before).unwrap();
        apply(&mut image, detour.aller.offset, &detour.aller.before).unwrap();
        assert_eq!(image, original, "et il se défait octet pour octet");
    }

    /// Une instruction relative à `rip` est RELOCALISÉE, et retombe sur
    /// exactement la même cible.
    ///
    /// C'est le contrôle qui compte le plus. Le code compilé moderne est
    /// plein de `mov rax, [rip+0x2e4f]` — c'est la forme normale d'un
    /// accès à une variable globale. Déplacée sans retoucher son écart,
    /// elle lirait ailleurs SANS RIEN SIGNALER, et la panne se
    /// manifesterait à trois sections de là.
    ///
    /// On ne vérifie donc pas les octets, mais la SOMME : l'adresse
    /// visée, recalculée depuis la nouvelle position, doit être celle
    /// d'avant. C'est la seule formulation qui ne puisse pas passer au
    /// vert par accident.
    #[test]
    fn a_rip_relative_instruction_is_relocated_to_the_same_target() {
        let mut image = vec![0u8; 0x200];
        // mov rax, [rip+0x2e4f] — 7 octets, à l'adresse 0x1000. Elle
        // vise donc 0x1000 + 7 + 0x2e4f = 0x3e56.
        image[0x00..0x07].copy_from_slice(&[0x48, 0x8B, 0x05, 0x4F, 0x2E, 0x00, 0x00]);
        let cible_avant = 0x1000u64 + 7 + 0x2e4f;

        let libre = place(0x1100, 0x100, 64);
        let detour = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[0x90], &[])
            .expect("une instruction relative à rip se relocalise");

        // Elle est réécrite : l'écart a changé.
        assert_eq!(detour.recouvert, 7);
        assert_ne!(
            detour.déplacé,
            vec![0x48, 0x8B, 0x05, 0x4F, 0x2E, 0x00, 0x00],
            "l'écart doit avoir été recalculé"
        );
        // Mais l'opcode et le modrm, non : on ne change pas ce qu'elle
        // fait, seulement d'où elle compte.
        assert_eq!(&detour.déplacé[..3], &[0x48, 0x8B, 0x05]);

        // Et la somme retombe au même endroit. L'ajout fait un octet,
        // donc elle vit maintenant en 0x1101.
        let nouvelle_ip = 0x1100u64 + 1;
        let écart = i32::from_le_bytes(detour.déplacé[3..7].try_into().unwrap());
        assert_eq!(
            (nouvelle_ip as i64 + 7 + écart as i64) as u64,
            cible_avant,
            "relocalisée, elle lit exactement la même adresse"
        );
    }

    /// Un `call` relatif est relocalisé de même, et appelle toujours la
    /// même fonction.
    #[test]
    fn a_relative_call_still_calls_the_same_function() {
        let mut image = vec![0u8; 0x200];
        // call rel32 en 0x1000 : vise 0x1000 + 5 + 0x10 = 0x1015.
        image[0x00..0x05].copy_from_slice(&[0xE8, 0x10, 0x00, 0x00, 0x00]);
        let cible_avant = 0x1015u64;

        let libre = place(0x1100, 0x100, 64);
        let detour = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[], &[])
            .expect("un call relatif se relocalise");

        assert_eq!(detour.déplacé[0], 0xE8, "c'est toujours un call");
        let écart = i32::from_le_bytes(detour.déplacé[1..5].try_into().unwrap());
        assert_eq!(
            (0x1100i64 + 5 + écart as i64) as u64,
            cible_avant,
            "relocalisé, il appelle la même fonction"
        );
    }

    /// Un saut COURT ne se relocalise pas : son champ d'un octet ne
    /// porte pas jusqu'à la place libre, et l'élargir changerait sa
    /// longueur.
    ///
    /// C'est la limite honnête de la méthode. Le refus dit la taille du
    /// champ, parce que c'est elle qui explique le non.
    #[test]
    fn a_short_jump_cannot_be_relocated_and_says_why() {
        let mut image = vec![0u8; 0x200];
        // jmp rel8 (2 octets) puis nop, nop, nop : le jmp rel32 du
        // détour recouvre les cinq octets, donc le saut court part.
        image[0x00..0x05].copy_from_slice(&[0xEB, 0x10, 0x90, 0x90, 0x90]);

        // La place libre est à 0x100 octets de là : hors de portée d'un
        // champ d'un seul octet.
        let libre = place(0x1100, 0x100, 64);
        let refus = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[], &[])
            .expect_err("un saut court ne peut pas suivre");

        match &refus {
            DetourRefusé::PortéeTropCourte {
                à,
                mnémonique,
                champ,
            } => {
                assert_eq!(*à, 0x1000);
                assert_eq!(mnémonique, "jmp");
                assert_eq!(*champ, 1, "un seul octet de portée");
            }
            autre => panic!("refus de portée attendu, obtenu {autre:?}"),
        }
        assert!(refus.message(true).contains("0x1000"));
        assert!(refus.message(false).contains("0x1000"));
    }

    /// Un saut court DANS la zone déplacée tient, si la place libre est
    /// assez proche.
    ///
    /// Le refus précédent n'est pas une règle sur les sauts courts, mais
    /// sur la distance. Ce test le prouve en rapprochant la place —
    /// sinon on aurait codé « jmp rel8 = refus » sans le savoir.
    #[test]
    fn a_short_jump_survives_when_the_slack_is_close_enough() {
        let mut image = vec![0u8; 0x200];
        image[0x00..0x05].copy_from_slice(&[0xEB, 0x10, 0x90, 0x90, 0x90]);
        let cible_avant = 0x1000u64 + 2 + 0x10;

        // Vingt octets plus loin : un champ d'un octet y suffit.
        let libre = place(0x1014, 0x14, 64);
        let detour = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[], &[])
            .expect("de près, le saut court se relocalise");

        assert_eq!(detour.déplacé[0], 0xEB);
        let écart = i8::from_le_bytes([detour.déplacé[1]]);
        assert_eq!(
            (0x1014i64 + 2 + écart as i64) as u64,
            cible_avant,
            "il saute toujours au même endroit"
        );
    }

    /// Un immédiat qui n'est PAS une cible ne doit pas être touché.
    ///
    /// `mov dword [rip+0x10], 0x42` porte les deux : un déplacement
    /// relatif à `rip`, qu'il faut recalculer, et un immédiat `0x42`,
    /// qui est une donnée. Confondre les deux ferait écrire une autre
    /// valeur — un défaut silencieux, et invisible à la relecture du
    /// listing.
    #[test]
    fn an_immediate_that_is_not_a_target_is_left_alone() {
        let mut image = vec![0u8; 0x200];
        // mov dword ptr [rip+0x10], 0x42 — 10 octets.
        image[0x00..0x0A]
            .copy_from_slice(&[0xC7, 0x05, 0x10, 0x00, 0x00, 0x00, 0x42, 0x00, 0x00, 0x00]);
        let cible_avant = 0x1000u64 + 10 + 0x10;

        let libre = place(0x1100, 0x100, 64);
        let detour =
            plan_detour(&image, 64, 0x00, 0x1000, &libre, &[], &[]).expect("elle se relocalise");

        // L'immédiat est intact.
        assert_eq!(
            &detour.déplacé[6..10],
            &[0x42, 0x00, 0x00, 0x00],
            "la valeur écrite ne change pas"
        );
        // Le déplacement, lui, a suivi.
        let écart = i32::from_le_bytes(detour.déplacé[2..6].try_into().unwrap());
        assert_eq!(
            (0x1100i64 + 10 + écart as i64) as u64,
            cible_avant,
            "et elle écrit toujours au même endroit"
        );
    }

    /// Ce qui ne dépend pas de sa position ressort identique.
    #[test]
    fn position_independent_code_is_copied_untouched() {
        let mut image = vec![0u8; 0x200];
        let original = [0xB8, 0x01, 0x00, 0x00, 0x00];
        image[0x00..0x05].copy_from_slice(&original);

        let libre = place(0x1100, 0x100, 64);
        let detour = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[], &[]).unwrap();
        assert_eq!(detour.déplacé, original.to_vec());
    }

    /// Une place libre dans les données est refusée la première, avant
    /// même qu'on décode quoi que ce soit.
    ///
    /// `slack::find` trie par taille, et les plus grands trous d'un
    /// binaire sont souvent dans `.rodata` : c'est donc le piège par
    /// défaut, celui qu'on tend à soi-même en prenant le premier de la
    /// liste.
    #[test]
    fn slack_in_a_data_segment_is_refused_first() {
        let mut image = vec![0u8; 0x200];
        image[0x00..0x05].copy_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);

        let mut données = place(0x1100, 0x100, 4096);
        données.executable = false;
        données.section = Some(".rodata".to_string());

        assert_eq!(
            plan_detour(&image, 64, 0x00, 0x1000, &données, &[0x90], &[]),
            Err(DetourRefusé::PlaceNonExécutable)
        );
    }

    /// Une place trop petite dit combien il aurait fallu.
    #[test]
    fn slack_too_small_says_how_much_was_needed() {
        let mut image = vec![0u8; 0x200];
        image[0x00..0x05].copy_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);

        // Besoin : 3 (ajout) + 5 (déplacé) + 5 (retour) = 13.
        let étroite = place(0x1100, 0x100, 12);
        assert_eq!(
            plan_detour(&image, 64, 0x00, 0x1000, &étroite, &[0x90, 0x90, 0x90], &[]),
            Err(DetourRefusé::PlaceTropPetite {
                besoin: 13,
                offert: 12
            })
        );
        // Un octet de plus, et il passe.
        let juste = place(0x1100, 0x100, 13);
        assert!(plan_detour(&image, 64, 0x00, 0x1000, &juste, &[0x90, 0x90, 0x90], &[]).is_ok());
    }

    /// Des instructions courtes sont recouvertes ENTIÈRES, et le reste
    /// comblé de `nop`.
    ///
    /// Trois `nop` d'un octet puis un `push rbp` : le `jmp` en demande
    /// cinq, donc il faut prendre les quatre instructions — six octets.
    /// Sans ce comportement, on couperait `push rbp` en deux et son
    /// second octet resterait, lisible comme autre chose.
    #[test]
    fn whole_instructions_are_covered_and_the_rest_is_padded() {
        let mut image = vec![0u8; 0x200];
        // nop, nop, nop, nop, nop, push rbp → le jmp couvre 5 nop pile.
        image[0x00..0x06].copy_from_slice(&[0x90, 0x90, 0x90, 0x90, 0x55, 0xC3]);

        let libre = place(0x1100, 0x100, 64);
        let detour = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[], &[]).unwrap();

        // Quatre `nop` font quatre octets, il en faut cinq : `push rbp`
        // entre aussi, ce qui porte à cinq.
        assert_eq!(detour.recouvert, 5);
        assert_eq!(detour.déplacé, vec![0x90, 0x90, 0x90, 0x90, 0x55]);
        assert_eq!(detour.aller.after.len(), 5);
        // Pile cinq octets : pas de nop de complément à ajouter.
        assert_eq!(detour.aller.after[0], 0xE9);
    }

    /// Une instruction longue fait dépasser, et le complément se voit.
    #[test]
    fn a_long_instruction_leaves_padding_behind() {
        let mut image = vec![0u8; 0x200];
        // mov rax, 0x1122334455667788 — 10 octets, sans rien de relatif.
        image[0x00..0x0A]
            .copy_from_slice(&[0x48, 0xB8, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]);

        let libre = place(0x1100, 0x100, 64);
        let detour = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[], &[]).unwrap();

        assert_eq!(detour.recouvert, 10, "l'instruction entière");
        assert_eq!(detour.aller.after.len(), 10);
        assert_eq!(
            &detour.aller.after[5..],
            &[0x90; 5],
            "cinq nop comblent le reste"
        );
    }

    /// L'endroit visé hors du fichier ne provoque pas de panique.
    #[test]
    fn a_site_outside_the_file_is_refused() {
        let image = vec![0u8; 0x10];
        let libre = place(0x1100, 0x00, 64);
        assert_eq!(
            plan_detour(&image, 64, 0x100, 0x1000, &libre, &[], &[]),
            Err(DetourRefusé::HorsFichier)
        );
    }

    /// Une place libre annoncée hors du fichier est refusée aussi — un
    /// binaire hostile peut décrire un segment qui déborde.
    #[test]
    fn slack_outside_the_file_is_refused() {
        let mut image = vec![0u8; 0x20];
        image[0x00..0x05].copy_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
        let menteuse = place(0x9000, 0x1000, 4096);
        assert_eq!(
            plan_detour(&image, 64, 0x00, 0x1000, &menteuse, &[], &[]),
            Err(DetourRefusé::HorsFichier)
        );
    }

    /// Un binaire où l'on a posé un premier détour, et la liste des
    /// correctifs qui va avec.
    ///
    /// Deux instructions de cinq octets, à 0x1000 et 0x1005, puis un
    /// `ret`. Le premier détour part de la première ; la seconde reste
    /// libre, et sert à montrer que la garde ne refuse pas tout.
    fn un_detour_deja_pose() -> (Vec<u8>, Vec<Patch>, crate::slack::Slack) {
        let mut image = vec![0u8; 0x400];
        // mov eax, 1 ; mov ecx, 2 ; ret
        image[0x00..0x05].copy_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
        image[0x05..0x0A].copy_from_slice(&[0xB9, 0x02, 0x00, 0x00, 0x00]);
        image[0x0A] = 0xC3;

        let libre = place(0x1100, 0x100, 64);
        let premier = plan_detour(&image, 64, 0x00, 0x1000, &libre, &[0x90], &[])
            .expect("le premier détour se pose");
        apply(&mut image, premier.corps.offset, &premier.corps.after).unwrap();
        apply(&mut image, premier.aller.offset, &premier.aller.after).unwrap();

        // Ce que l'application garde : les deux écritures, dans l'ordre
        // où elle les empile.
        let posés = vec![premier.corps.clone(), premier.aller.clone()];

        // La place libre telle qu'elle reste : `slack::find` reconnaît
        // le bourrage à lui-même, donc le corps écrit n'en est plus. On
        // reprend donc la queue du trou.
        let pris = premier.corps.after.len() as u64;
        let reste = place(0x1100 + pris, 0x100 + pris, 64 - pris);
        (image, posés, reste)
    }

    /// Détourner deux fois le même endroit écrasait le premier détour,
    /// en silence.
    ///
    /// Le piège n'est pas théorique : un crochet Frida posé sur `main`
    /// a fait exactement cela à un détour de namsdec, et la raison vaut
    /// des deux côtés — cinq octets de saut, un seul endroit où les
    /// mettre. Mais vu de l'image seule, il n'y a rien à voir : le `jmp`
    /// du premier détour se décode comme n'importe quel `jmp`, il se
    /// relocalise très bien, et le plan sort sans se plaindre.
    ///
    /// Le contrôle MONTRE le défaut avant de montrer la garde, dans le
    /// même corps : sans la liste des correctifs, le second plan vise
    /// les mêmes octets que le premier — c'est-à-dire qu'il les écrase.
    #[test]
    fn a_second_detour_does_not_quietly_overwrite_the_first() {
        let (image, posés, reste) = un_detour_deja_pose();

        // LE DÉFAUT, remis sous les yeux : sans la liste, le plan sort.
        let aveugle = plan_detour(&image, 64, 0x00, 0x1000, &reste, &[0x90], &[])
            .expect("sans la liste, rien ne s'y oppose");
        assert_eq!(
            aveugle.aller.offset, posés[1].offset,
            "le second aller écrirait sur les octets du premier : \
             position {:#x} contre {:#x}",
            aveugle.aller.offset, posés[1].offset
        );

        // LA GARDE : la même demande, avec ce qui est déjà posé.
        assert_eq!(
            plan_detour(&image, 64, 0x00, 0x1000, &reste, &[0x90], &posés),
            Err(DetourRefusé::SurUnCorrectif {
                à: 0x1000,
                offset: 0x00,
                correctif: 0x1000,
            }),
            "un site déjà détourné se refuse, et nomme le correctif"
        );
    }

    /// Le refus ne demande pas que les deux sites coïncident : un seul
    /// octet commun suffit à perdre les deux correctifs.
    ///
    /// C'est le cas le PLUS dangereux, et le moins visible : viser un
    /// octet après le site d'un détour réécrit le déplacement du `jmp`
    /// à moitié. Le premier détour ne redevient pas inoffensif — il
    /// saute quelque part, et ce quelque part n'existe pas.
    #[test]
    fn overlapping_a_patch_by_one_byte_is_refused_too() {
        let (image, posés, reste) = un_detour_deja_pose();

        for décalage in 1..JMP_REL32 {
            let site = 0x1000 + décalage as u64;
            let refus = plan_detour(&image, 64, décalage, site, &reste, &[], &posés);
            assert_eq!(
                refus,
                Err(DetourRefusé::SurUnCorrectif {
                    à: site,
                    offset: décalage,
                    correctif: 0x1000,
                }),
                "un détour en {site:#x} tombe encore dans le `jmp` posé en 0x1000"
            );
        }
    }

    /// Et la garde ne refuse pas tout : un endroit libre reste posable.
    ///
    /// Un contrôle qui ne dirait que « ça refuse » passerait au vert
    /// avec un refus inconditionnel, c'est-à-dire avec la
    /// fonctionnalité éteinte. C'est lui qui donne son sens au
    /// précédent.
    #[test]
    fn a_detour_elsewhere_is_still_allowed() {
        let (image, posés, reste) = un_detour_deja_pose();

        // 0x1005 : la seconde instruction, hors de tout ce qui est posé.
        let second = plan_detour(&image, 64, 0x05, 0x1005, &reste, &[0x90], &posés)
            .expect("rien n'est posé là : ce détour tient");
        assert_eq!(second.recouvert, 5, "`mov ecx, 2` fait cinq octets");
        assert!(
            chevauche(&posés, second.aller.offset, second.aller.after.len()).is_none(),
            "et il n'empiète sur rien"
        );
        assert!(
            chevauche(&posés, second.corps.offset, second.corps.after.len()).is_none(),
            "son corps non plus"
        );
    }

    /// Le corps compte autant que l'aller : écrire par-dessus le corps
    /// d'un premier détour le prive de son code sans toucher à son saut.
    ///
    /// Le programme continue alors d'y sauter, et exécute ce que le
    /// second y a mis — un détour qui part, et ne revient pas où il
    /// croyait.
    #[test]
    fn writing_over_the_body_of_a_detour_is_refused() {
        let (image, posés, _) = un_detour_deja_pose();

        // La place libre reprise depuis son DÉBUT, c'est-à-dire là où le
        // corps du premier détour est écrit. `slack::find` ne l'offrirait
        // plus, mais un plan calculé avant lui, si.
        let occupée = place(0x1100, 0x100, 64);
        let refus = plan_detour(&image, 64, 0x05, 0x1005, &occupée, &[0x90], &posés);
        assert_eq!(
            refus,
            Err(DetourRefusé::SurUnCorrectif {
                à: 0x1100,
                offset: 0x100,
                correctif: 0x1100,
            }),
            "le corps du premier détour n'est pas de la place libre"
        );
    }

    /// `chevauche` compare les octets ÉCRITS, bornes comprises d'un
    /// côté et exclues de l'autre.
    ///
    /// Une erreur d'un octet ici refuserait un détour parfaitement
    /// posable, ou en laisserait passer un qui mord sur le voisin.
    #[test]
    fn overlap_is_measured_on_the_bytes_actually_written() {
        let posés = vec![Patch {
            offset: 0x10,
            before: vec![0; 4],
            after: vec![0x90; 4],
            addr: 0x2000,
            group: None,
        }];
        // Juste avant, bord à bord : pas de chevauchement.
        assert!(chevauche(&posés, 0x0C, 4).is_none());
        // Un octet de plus, et il mord.
        assert!(chevauche(&posés, 0x0C, 5).is_some());
        // Juste après, bord à bord.
        assert!(chevauche(&posés, 0x14, 4).is_none());
        // Le dernier octet écrit.
        assert!(chevauche(&posés, 0x13, 1).is_some());
        // Un correctif qui n'écrit rien n'occupe rien.
        let vide = vec![Patch {
            offset: 0x10,
            before: vec![0; 4],
            after: Vec::new(),
            addr: 0x2000,
            group: None,
        }];
        assert!(chevauche(&vide, 0x10, 4).is_none());
    }
}
