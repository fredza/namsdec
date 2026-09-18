//! Lecture de l'ELF64 — ce qu'il faut pour charger un programme en
//! mémoire simulée comme le noyau le ferait, et relier ses lignes source
//! à des adresses.
//!
//! Deux lectures cohabitent, et elles ne servent pas à la même chose.
//!
//! Les **en-têtes de programme** disent ce qu'il faut *charger* : des
//! segments, avec leurs droits et leur place en mémoire. C'est la seule
//! liste que le noyau regarde, et c'est celle qui survit à `strip`.
//!
//! Les **sections** disent ce qu'il y a *dedans* : `.text`, `.got`,
//! `.rodata`. Elles ne décident de rien au chargement, mais elles
//! nomment ce qu'on a chargé — et savoir qu'on vient de tomber dans
//! `.got` plutôt que « dans des données » est souvent toute la réponse à
//! la question qu'on se pose.
//!
//! S'y ajoute la **table dynamique**, que lit d'ordinaire l'éditeur de
//! liens dynamique : les relocations à appliquer avant la première
//! instruction, et les symboles qu'il faudrait aller chercher ailleurs.
//! Sans elles, un binaire moderne se charge sur des tables de pointeurs
//! entièrement nulles — et part dans le décor au premier appel indirect,
//! sans que rien n'explique pourquoi.
//!
//! Il lit aussi la table des symboles, quand `ld` en laisse une : ce sont
//! les étiquettes écrites dans le source, et les revoir en face d'une
//! adresse est toute la différence entre « 0x401002 » et « tampon+2 ».
//!
//! Ce qu'il ne fait pas : charger l'interpréteur, résoudre un symbole de
//! bibliothèque, appeler un résolveur `ifunc`. Ce sont là les travaux de
//! `ld.so`, qui est un programme à lui seul.

/// Une section chargeable du programme lié.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    /// Adresse virtuelle où la section doit se trouver en mémoire.
    pub addr: u64,
    /// Contenu. Vide et de taille `size` pour `.bss`, qui n'occupe pas de
    /// place dans le fichier mais bien en mémoire.
    pub data: Vec<u8>,
    pub size: usize,
    pub executable: bool,
    /// `.data` et `.bss` s'écrivent ; `.text` et `.rodata` non. C'est ce
    /// qui permet de refuser à un programme d'écrire dans son propre
    /// code plutôt que de le laisser faire en silence.
    pub writable: bool,
}

/// Une étiquette du programme, telle qu'elle est écrite dans le source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub addr: u64,
    /// La taille déclarée, souvent nulle : nasm ne la renseigne que pour
    /// les symboles passés par `size`.
    pub size: u64,
}

/// Un segment de chargement, tel que l'en-tête de programme le décrit.
///
/// C'est la vue du noyau : une tranche du fichier à poser à une adresse,
/// avec des droits. `memsz` dépasse `filesz` quand le segment se termine
/// par du `.bss` — de la mémoire qui existe sans rien peser dans le
/// fichier, et que le chargeur met à zéro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub vaddr: u64,
    /// Où les octets commencent dans le fichier.
    pub offset: usize,
    /// Combien d'octets viennent du fichier.
    pub filesz: usize,
    /// Combien de place le segment occupe en mémoire. Le surplus est
    /// mis à zéro par le chargeur : c'est `.bss`.
    pub memsz: usize,
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub align: u64,
}

/// Le modèle de mémoire locale au fil d'exécution, décrit par `PT_TLS`.
///
/// Le chargeur en fait une copie par fil, et installe la base FS dessus.
/// Un programme moderne y garde son canari de pile : sans ce bloc, la
/// toute première fonction protégée lit une adresse qui n'existe pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tls {
    pub vaddr: u64,
    pub filesz: usize,
    pub memsz: usize,
    pub align: u64,
}

/// Ce que le fichier dit être.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    /// Un `.o` : `ld` n'est pas encore passé, rien n'a d'adresse.
    Relocatable,
    /// Un exécutable à position fixe, chargé là où il le dit.
    #[default]
    Executable,
    /// Un objet partagé — une bibliothèque, ou un exécutable indépendant
    /// de la position (PIE), ce qui est le cas de presque tout ce qu'une
    /// distribution livre aujourd'hui.
    Shared,
    Other(u16),
}

/// Ce qu'une relocation demande.
///
/// Il y en a des dizaines ; celles-ci sont celles qu'un exécutable x86-64
/// porte vraiment. Les autres sont gardées telles quelles pour pouvoir
/// être *comptées et nommées* plutôt que passées sous silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocKind {
    /// « Ajoute la base de chargement » : la relocation de tout binaire
    /// indépendant de la position, et de très loin la plus nombreuse.
    Relative,
    /// Une entrée de table de liaison : l'adresse d'une fonction
    /// extérieure, que `ld.so` remplit au premier appel.
    JumpSlot,
    /// Une case de données qui doit contenir l'adresse d'un symbole.
    GlobDat,
    /// Un mot de 64 bits à remplir avec l'adresse d'un symbole.
    Direct64,
    /// Une fonction choisie à l'exécution selon le processeur : c'est un
    /// *résolveur* qu'il faut appeler pour obtenir la vraie adresse.
    Irelative,
    /// Une copie de donnée depuis une bibliothèque.
    Copy,
    /// Un décalage dans le bloc TLS.
    TlsOffset,
    Other(u32),
}

impl RelocKind {
    fn from_type(kind: u32) -> Self {
        match kind {
            R_X86_64_64 => Self::Direct64,
            R_X86_64_COPY => Self::Copy,
            R_X86_64_GLOB_DAT => Self::GlobDat,
            R_X86_64_JUMP_SLOT => Self::JumpSlot,
            R_X86_64_RELATIVE => Self::Relative,
            R_X86_64_TPOFF64 => Self::TlsOffset,
            R_X86_64_IRELATIVE => Self::Irelative,
            autre => Self::Other(autre),
        }
    }

    /// Le nom court de la relocation, tel que `readelf` l'écrit — pour
    /// pouvoir dire *laquelle* n'a pas été appliquée.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Relative => "R_X86_64_RELATIVE",
            Self::JumpSlot => "R_X86_64_JUMP_SLOT",
            Self::GlobDat => "R_X86_64_GLOB_DAT",
            Self::Direct64 => "R_X86_64_64",
            Self::Irelative => "R_X86_64_IRELATIVE",
            Self::Copy => "R_X86_64_COPY",
            Self::TlsOffset => "R_X86_64_TPOFF64",
            Self::Other(_) => "R_X86_64_?",
        }
    }
}

/// Une relocation : une case à remplir, et de quoi la remplir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reloc {
    /// L'adresse de la case à corriger, dans les coordonnées du fichier.
    pub at: u64,
    pub kind: RelocKind,
    pub addend: i64,
    /// Le symbole visé, quand la relocation en nomme un. Vide pour les
    /// relocations purement relatives, qui n'ont besoin de personne.
    pub symbol: String,
    /// La valeur du symbole dans *ce* fichier. Nulle quand il est
    /// indéfini — c'est alors une bibliothèque qui devrait le fournir.
    pub symbol_value: u64,
    /// La valeur à corriger est-elle déjà dans la case ?
    ///
    /// C'est le cas du format compact `DT_RELR`, qui ne stocke aucun
    /// addend : la case contient déjà l'adresse telle qu'elle serait si
    /// le programme était chargé à zéro, et le chargeur n'a qu'à y
    /// ajouter la base.
    pub implicit: bool,
}

impl Reloc {
    /// La relocation attend-elle quelque chose que seule une bibliothèque
    /// pourrait donner ?
    pub fn external(&self) -> bool {
        !self.symbol.is_empty() && self.symbol_value == 0
    }
}

/// Ce que le relogement a fait, et ce qu'il n'a pas pu faire.
///
/// Un chargeur silencieux est un chargeur qu'on ne peut pas mettre en
/// cause : ce rapport est ce que namsdec montre après avoir ouvert un
/// binaire, et c'est souvent lui qui explique la suite.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelocReport {
    /// Combien de cases ont été remplies.
    pub applied: usize,
    /// Les noms des fonctions et données qu'il aurait fallu aller
    /// chercher dans une bibliothèque, triés et sans doublon.
    pub external: Vec<String>,
    /// Combien de fonctions sont choisies à l'exécution par un résolveur
    /// (`ifunc`) — `memcpy` et ses semblables, dans toute libc moderne.
    pub ifunc: usize,
    /// Les relocations laissées de côté, par type.
    pub skipped: Vec<RelocKind>,
    /// Combien visaient une case hors de l'image chargée. Signe d'un
    /// fichier abîmé, ou d'une table qu'on a mal lue.
    pub outside: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Elf {
    pub entry: u64,
    pub kind: Kind,
    pub sections: Vec<Section>,
    /// Les étiquettes du programme, triées par adresse croissante.
    pub symbols: Vec<Symbol>,
    /// Ce qu'il faut charger, dans l'ordre du fichier.
    pub segments: Vec<Segment>,
    /// Le modèle TLS, s'il y en a un.
    pub tls: Option<Tls>,
    /// Les relocations lues dans la table dynamique — celles que `ld.so`
    /// appliquerait.
    pub relocs: Vec<Reloc>,
    /// Où la table des en-têtes de programme se trouve **en mémoire**, et
    /// comment elle est découpée. Le programme la relit par `AT_PHDR` :
    /// c'est ainsi qu'il retrouve son propre modèle TLS.
    pub phdr: Option<(u64, usize, usize)>,
    /// Le fichier lui-même.
    ///
    /// Un chargeur a besoin des octets et pas seulement de la carte : les
    /// segments disent « telle tranche du fichier va là », et la tranche
    /// doit être quelque part. La garder ici évite d'avoir à repasser le
    /// tampon d'origine à chaque appel — et surtout d'avoir à se demander
    /// si c'est bien le même.
    pub file: Vec<u8>,
}

impl Kind {
    /// Indépendant de la position : ses adresses sont des décalages
    /// depuis une base que le chargeur choisit.
    pub fn pie(&self) -> bool {
        matches!(self, Self::Shared)
    }
}

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_TLS: u32 = 7;
/// Les droits d'un segment, tels que l'en-tête de programme les porte.
const PF_X: u32 = 0x1;
const PF_W: u32 = 0x2;
const PF_R: u32 = 0x4;

/// Les entrées de la table dynamique dont dépend le chargement.
const DT_NULL: u64 = 0;
const DT_STRTAB: u64 = 5;
const DT_SYMTAB: u64 = 6;
const DT_RELA: u64 = 7;
const DT_RELASZ: u64 = 8;
const DT_RELAENT: u64 = 9;
const DT_STRSZ: u64 = 10;
const DT_SYMENT: u64 = 11;
const DT_PLTRELSZ: u64 = 2;
const DT_JMPREL: u64 = 23;
/// `DT_RELR` : le format compact des relocations relatives, apparu en
/// 2022. Un binaire récent y range l'essentiel de son relogement, et qui
/// ne le lit pas croit n'avoir rien à faire.
const DT_RELR: u64 = 36;
const DT_RELRSZ: u64 = 35;

/// Les types de relocation d'x86-64 que le chargeur rencontre.
const R_X86_64_64: u32 = 1;
const R_X86_64_COPY: u32 = 5;
const R_X86_64_GLOB_DAT: u32 = 6;
const R_X86_64_JUMP_SLOT: u32 = 7;
const R_X86_64_RELATIVE: u32 = 8;
const R_X86_64_TPOFF64: u32 = 18;
const R_X86_64_IRELATIVE: u32 = 37;

/// La taille d'une entrée `Elf64_Rela` et d'un symbole dynamique.
const RELA_SIZE: usize = 24;
const SYM_SIZE: usize = 24;

/// La page : l'unité dans laquelle un noyau projette la mémoire. Un
/// segment qui commence au milieu d'une page fait tout de même projeter
/// la page entière — et c'est pour cela qu'un programme peut lire les
/// quelques octets qui suivent son dernier segment sans rien casser.
pub const PAGE: u64 = 0x1000;

const SHF_ALLOC: u64 = 0x2;
const SHF_WRITE: u64 = 0x1;
const SHF_EXECINSTR: u64 = 0x4;
/// `.bss` : la section occupe de la mémoire mais rien dans le fichier.
const SHT_NOBITS: u32 = 8;
/// La table des symboles laissée par l'éditeur de liens.
const SHT_SYMTAB: u32 = 2;
/// Types de symboles qui désignent quelque chose en mémoire : une
/// donnée, une fonction, ou une étiquette sans type — le cas ordinaire
/// en assembleur.
const STT_NOTYPE: u8 = 0;
const STT_OBJECT: u8 = 1;
const STT_FUNC: u8 = 2;
/// L'index de section « aucune » : un symbole absolu ou indéfini, qui ne
/// nomme pas une adresse du programme.
const SHN_UNDEF: u16 = 0;
const SHN_ABS: u16 = 0xfff1;

/// L'adresse ramenée au début de sa page.
pub fn page_down(addr: u64) -> u64 {
    addr & !(PAGE - 1)
}

/// L'adresse arrondie au début de la page suivante — elle-même si elle
/// est déjà alignée.
pub fn page_up(addr: u64) -> u64 {
    addr.div_ceil(PAGE).saturating_mul(PAGE)
}

/// Arrondit vers le haut sur un alignement quelconque, puissance de deux
/// ou non — le champ `p_align` d'un segment TLS vaut parfois 32.
pub fn align_up(value: u64, align: u64) -> u64 {
    let align = align.max(1);
    value.div_ceil(align).saturating_mul(align)
}

/// Lit un mot de 64 bits dans l'image, à une adresse *virtuelle*.
fn read_u64(image: &[u8], image_addr: u64, at: u64) -> Option<u64> {
    let offset = at.checked_sub(image_addr)? as usize;
    let bytes = image.get(offset..offset.checked_add(8)?)?;
    Some(u64::from_le_bytes(bytes.try_into().ok()?))
}

/// Écrit un mot de 64 bits dans l'image, à une adresse *virtuelle*. Rend
/// `false` quand l'adresse tombe hors de ce qui a été chargé.
fn write_u64(image: &mut [u8], image_addr: u64, at: u64, value: u64) -> bool {
    let Some(offset) = at.checked_sub(image_addr).map(|o| o as usize) else {
        return false;
    };
    let Some(case) = image.get_mut(offset..offset.saturating_add(8)) else {
        return false;
    };
    case.copy_from_slice(&value.to_le_bytes());
    true
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn u64_at(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

/// Lit l'en-tête, les segments à charger, la table dynamique et les
/// sections. Rend `None` sur ce qui n'est pas un ELF64 petit-boutiste.
///
/// Tout le reste est *facultatif* : un binaire dépouillé n'a plus ni
/// sections ni symboles et se charge parfaitement — c'est même le cas
/// ordinaire de ce qu'on ouvre pour l'étudier. Refuser un fichier parce
/// qu'il manque une table qui ne sert qu'à le lire confortablement
/// reviendrait à confondre le confort et la possibilité.
pub fn parse(bytes: &[u8]) -> Option<Elf> {
    if bytes.get(..4)? != b"\x7fELF" {
        return None;
    }
    // EI_CLASS = 2 (64 bits), EI_DATA = 1 (petit-boutiste).
    if *bytes.get(4)? != 2 || *bytes.get(5)? != 1 {
        return None;
    }

    let kind = match u16_at(bytes, 0x10)? {
        1 => Kind::Relocatable,
        2 => Kind::Executable,
        3 => Kind::Shared,
        autre => Kind::Other(autre),
    };
    let entry = u64_at(bytes, 0x18)?;
    let (segments, tls, dynamic_at, phdr) = parse_program_headers(bytes).unwrap_or_default();
    let sections = parse_sections(bytes).unwrap_or_default();

    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    let symbols = parse_symbols(bytes, shoff, shentsize, shnum).unwrap_or_default();

    // Les relocations se lisent dans la table dynamique — celle que
    // `ld.so` consulte — et non dans les sections : c'est la seule qui
    // survit à `strip`, et la seule que le programme lui-même désigne.
    let relocs = dynamic_at
        .and_then(|(offset, size)| parse_dynamic_relocs(bytes, &segments, offset, size))
        .unwrap_or_default();

    Some(Elf {
        entry,
        kind,
        sections,
        symbols,
        segments,
        tls,
        relocs,
        phdr,
        file: bytes.to_vec(),
    })
}

/// Les segments à charger, le modèle TLS, où trouver la table dynamique,
/// et où la table des en-têtes se projette elle-même.
///
/// C'est exactement ce que le noyau lit, et dans cet ordre.
#[allow(clippy::type_complexity)]
fn parse_program_headers(
    bytes: &[u8],
) -> Option<(
    Vec<Segment>,
    Option<Tls>,
    Option<(usize, usize)>,
    Option<(u64, usize, usize)>,
)> {
    let phoff = u64_at(bytes, 0x20)? as usize;
    let phentsize = u16_at(bytes, 0x36)? as usize;
    let phnum = u16_at(bytes, 0x38)? as usize;
    if phoff == 0 || phentsize < 56 || phnum == 0 {
        // Un `.o` n'a pas d'en-têtes de programme : il n'est pas
        // chargeable, et c'est précisément ce qu'on veut savoir.
        return None;
    }

    let mut segments = Vec::new();
    let mut tls = None;
    let mut dynamic_at = None;
    for index in 0..phnum {
        let at = phoff.checked_add(index.checked_mul(phentsize)?)?;
        let kind = u32_at(bytes, at)?;
        let flags = u32_at(bytes, at + 0x04)?;
        let offset = u64_at(bytes, at + 0x08)? as usize;
        let vaddr = u64_at(bytes, at + 0x10)?;
        let filesz = u64_at(bytes, at + 0x20)? as usize;
        let memsz = u64_at(bytes, at + 0x28)? as usize;
        let align = u64_at(bytes, at + 0x30)?;
        match kind {
            PT_LOAD => segments.push(Segment {
                vaddr,
                offset,
                // Un `filesz` qui déborde du fichier est une erreur de
                // l'écrivain, pas une raison de refuser le reste : on
                // charge ce qui existe.
                filesz: filesz.min(bytes.len().saturating_sub(offset)),
                memsz: memsz.max(filesz),
                read: flags & PF_R != 0,
                write: flags & PF_W != 0,
                execute: flags & PF_X != 0,
                align: align.max(1),
            }),
            PT_TLS => {
                tls = Some(Tls {
                    vaddr,
                    filesz,
                    memsz: memsz.max(filesz),
                    align: align.max(1),
                })
            }
            PT_DYNAMIC => dynamic_at = Some((offset, filesz)),
            _ => {}
        }
    }
    segments.sort_by_key(|segment| segment.vaddr);

    // Où la table des en-têtes atterrit en mémoire : c'est le segment
    // qui la couvre qui le dit. Le programme la relit par `AT_PHDR`
    // pour retrouver, entre autres, son propre modèle TLS.
    let phdr = segments
        .iter()
        .find(|segment| {
            phoff >= segment.offset && phoff + phentsize * phnum <= segment.offset + segment.filesz
        })
        .map(|segment| {
            (
                segment.vaddr + (phoff - segment.offset) as u64,
                phentsize,
                phnum,
            )
        });

    Some((segments, tls, dynamic_at, phdr))
}

/// Les sections présentes en mémoire à l'exécution, avec leur contenu.
///
/// Elles ne décident de rien — c'est le rôle des segments — mais elles
/// nomment ce qui a été chargé.
fn parse_sections(bytes: &[u8]) -> Option<Vec<Section>> {
    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    let shstrndx = u16_at(bytes, 0x3e)? as usize;
    if shoff == 0 || shnum == 0 || shstrndx >= shnum {
        return None;
    }

    // La table des noms de sections est elle-même une section : il faut la
    // lire en premier pour pouvoir nommer les autres.
    let shstr_header = shoff.checked_add(shstrndx.checked_mul(shentsize)?)?;
    let shstr_offset = u64_at(bytes, shstr_header + 0x18)? as usize;
    let shstr_size = u64_at(bytes, shstr_header + 0x20)? as usize;
    let names = bytes.get(shstr_offset..shstr_offset.checked_add(shstr_size)?)?;

    let mut sections = Vec::new();
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        let flags = u64_at(bytes, header + 0x08)?;
        // Seules les sections présentes en mémoire à l'exécution nous
        // intéressent : les tables de symboles et de chaînes n'ont pas
        // d'adresse et n'ont rien à faire dans la mémoire simulée.
        if flags & SHF_ALLOC == 0 {
            continue;
        }

        let name_offset = u32_at(bytes, header)? as usize;
        let name = names
            .get(name_offset..)?
            .iter()
            .position(|&b| b == 0)
            .map(|end| String::from_utf8_lossy(&names[name_offset..name_offset + end]).into_owned())
            .unwrap_or_default();

        let kind = u32_at(bytes, header + 0x04)?;
        let addr = u64_at(bytes, header + 0x10)?;
        let offset = u64_at(bytes, header + 0x18)? as usize;
        let size = u64_at(bytes, header + 0x20)? as usize;

        let data = if kind == SHT_NOBITS {
            Vec::new()
        } else {
            bytes.get(offset..offset.checked_add(size)?)?.to_vec()
        };

        sections.push(Section {
            name,
            addr,
            data,
            size,
            executable: flags & SHF_EXECINSTR != 0,
            writable: flags & SHF_WRITE != 0,
        });
    }
    Some(sections)
}

/// L'adresse virtuelle `vaddr`, ramenée à un décalage dans le fichier.
///
/// La table dynamique parle en adresses ; le fichier, en décalages. Ce
/// sont les segments qui font la conversion — et eux seuls.
fn file_offset(segments: &[Segment], vaddr: u64) -> Option<usize> {
    segments.iter().find_map(|segment| {
        let fin = segment.vaddr.checked_add(segment.filesz as u64)?;
        (vaddr >= segment.vaddr && vaddr < fin)
            .then(|| segment.offset + (vaddr - segment.vaddr) as usize)
    })
}

/// Les relocations que porte la table dynamique.
///
/// Trois listes s'y côtoient, et il faut les trois : `DT_RELA` pour les
/// données, `DT_JMPREL` pour la table de liaison des fonctions, et
/// `DT_RELR` pour le format compact des relocations relatives, où un
/// seul mot de 64 bits en décrit soixante-trois.
fn parse_dynamic_relocs(
    bytes: &[u8],
    segments: &[Segment],
    offset: usize,
    size: usize,
) -> Option<Vec<Reloc>> {
    let mut valeurs: Vec<(u64, u64)> = Vec::new();
    for at in (offset..offset.checked_add(size)?).step_by(16) {
        let tag = u64_at(bytes, at)?;
        let value = u64_at(bytes, at + 8)?;
        if tag == DT_NULL {
            break;
        }
        valeurs.push((tag, value));
    }
    let valeur = |tag: u64| valeurs.iter().find(|(t, _)| *t == tag).map(|(_, v)| *v);

    let strtab = valeur(DT_STRTAB).and_then(|addr| file_offset(segments, addr));
    let strsz = valeur(DT_STRSZ).unwrap_or(0) as usize;
    let symtab = valeur(DT_SYMTAB).and_then(|addr| file_offset(segments, addr));
    let syment = valeur(DT_SYMENT).unwrap_or(SYM_SIZE as u64) as usize;

    let mut relocs = Vec::new();
    // `DT_RELA` (les données) et `DT_JMPREL` (la table de liaison) ont le
    // même format : des entrées de 24 octets.
    for (adresse, taille) in [
        (valeur(DT_RELA), valeur(DT_RELASZ)),
        (valeur(DT_JMPREL), valeur(DT_PLTRELSZ)),
    ] {
        let (Some(adresse), Some(taille)) = (adresse, taille) else {
            continue;
        };
        let entsize = valeur(DT_RELAENT).unwrap_or(RELA_SIZE as u64).max(1) as usize;
        let Some(début) = file_offset(segments, adresse) else {
            continue;
        };
        for at in (début..début.saturating_add(taille as usize)).step_by(entsize) {
            let Some(r_offset) = u64_at(bytes, at) else {
                break;
            };
            let Some(info) = u64_at(bytes, at + 8) else {
                break;
            };
            let Some(addend) = u64_at(bytes, at + 16) else {
                break;
            };
            let index = (info >> 32) as usize;
            let kind = RelocKind::from_type((info & 0xffff_ffff) as u32);
            let (symbol, symbol_value) = match (symtab, index) {
                (Some(symtab), index) if index > 0 => {
                    dynamic_symbol(bytes, symtab + index * syment, strtab, strsz)
                        .unwrap_or_default()
                }
                _ => (String::new(), 0),
            };
            relocs.push(Reloc {
                at: r_offset,
                kind,
                addend: addend as i64,
                symbol,
                symbol_value,
                implicit: false,
            });
        }
    }

    // `DT_RELR` : une adresse, puis des masques de bits. Chaque bit à 1
    // dit « la case suivante aussi ». C'est compact au point qu'un
    // binaire récent y range presque tout son relogement.
    if let (Some(adresse), Some(taille)) = (valeur(DT_RELR), valeur(DT_RELRSZ))
        && let Some(début) = file_offset(segments, adresse)
    {
        let mut curseur = 0u64;
        for at in (début..début.saturating_add(taille as usize)).step_by(8) {
            let Some(mot) = u64_at(bytes, at) else { break };
            if mot & 1 == 0 {
                // Une adresse : elle est relogée, et la suite se compte
                // à partir de la case d'après.
                relocs.push(Reloc {
                    at: mot,
                    kind: RelocKind::Relative,
                    addend: 0,
                    symbol: String::new(),
                    symbol_value: 0,
                    implicit: true,
                });
                curseur = mot.saturating_add(8);
            } else {
                let mut bits = mot >> 1;
                let mut place = curseur;
                while bits != 0 {
                    if bits & 1 != 0 {
                        relocs.push(Reloc {
                            at: place,
                            kind: RelocKind::Relative,
                            addend: 0,
                            symbol: String::new(),
                            symbol_value: 0,
                            implicit: true,
                        });
                    }
                    bits >>= 1;
                    place = place.saturating_add(8);
                }
                curseur = curseur.saturating_add(63 * 8);
            }
        }
    }

    Some(relocs)
}

/// Le nom et la valeur d'un symbole dynamique, lus à son entrée.
fn dynamic_symbol(
    bytes: &[u8],
    at: usize,
    strtab: Option<usize>,
    strsz: usize,
) -> Option<(String, u64)> {
    let name_offset = u32_at(bytes, at)? as usize;
    let value = u64_at(bytes, at + 8)?;
    let strtab = strtab?;
    let names = bytes.get(strtab..strtab.checked_add(strsz)?)?;
    let name = names
        .get(name_offset..)?
        .iter()
        .position(|&b| b == 0)
        .map(|end| String::from_utf8_lossy(&names[name_offset..name_offset + end]).into_owned())
        .unwrap_or_default();
    Some((name, value))
}

/// Les étiquettes du programme, lues dans `.symtab`.
///
/// Rend `None` — et non une erreur — quand il n'y en a pas : un binaire
/// dépouillé (`strip`) se charge et s'exécute très bien, il se lit
/// seulement moins agréablement.
fn parse_symbols(
    bytes: &[u8],
    shoff: usize,
    shentsize: usize,
    shnum: usize,
) -> Option<Vec<Symbol>> {
    let mut symbols = Vec::new();
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        if u32_at(bytes, header + 0x04)? != SHT_SYMTAB {
            continue;
        }
        // `sh_link` d'une table de symboles donne la section de chaînes
        // où sont leurs noms.
        let strtab = u32_at(bytes, header + 0x28)? as usize;
        let strtab_header = shoff.checked_add(strtab.checked_mul(shentsize)?)?;
        let str_offset = u64_at(bytes, strtab_header + 0x18)? as usize;
        let str_size = u64_at(bytes, strtab_header + 0x20)? as usize;
        let names = bytes.get(str_offset..str_offset.checked_add(str_size)?)?;

        let offset = u64_at(bytes, header + 0x18)? as usize;
        let size = u64_at(bytes, header + 0x20)? as usize;
        let entry_size = u64_at(bytes, header + 0x38)? as usize;
        if entry_size == 0 {
            continue;
        }
        for at in (offset..offset.checked_add(size)?).step_by(entry_size) {
            let name_offset = u32_at(bytes, at)? as usize;
            let info = *bytes.get(at + 4)?;
            let shndx = u16_at(bytes, at + 6)?;
            let value = u64_at(bytes, at + 8)?;
            let sym_size = u64_at(bytes, at + 16)?;
            // Le type est dans les quatre bits bas de st_info.
            let kind = info & 0xf;
            if !matches!(kind, STT_NOTYPE | STT_OBJECT | STT_FUNC) {
                continue;
            }
            if shndx == SHN_UNDEF || shndx == SHN_ABS {
                continue;
            }
            let name = names
                .get(name_offset..)?
                .iter()
                .position(|&b| b == 0)
                .map(|end| {
                    String::from_utf8_lossy(&names[name_offset..name_offset + end]).into_owned()
                })
                .unwrap_or_default();
            // Les symboles que produit l'assembleur pour son propre usage
            // — le nom du fichier, les étiquettes locales `.L…` — ne
            // disent rien à qui lit son programme.
            if name.is_empty() || name.starts_with(".L") || name.ends_with(".asm") {
                continue;
            }
            symbols.push(Symbol {
                name,
                addr: value,
                size: sym_size,
            });
        }
    }
    symbols.sort_by_key(|s| (s.addr, s.name.clone()));
    symbols.dedup();
    Some(symbols)
}

impl Elf {
    /// L'adresse de la section nommée, pour rattacher à une adresse réelle
    /// les décalages du listing de nasm, qui sont relatifs à leur section.
    pub fn section_addr(&self, name: &str) -> Option<u64> {
        self.sections
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.addr)
    }

    /// L'image telle qu'un noyau la pose : par **segments**, en pages
    /// entières, le `.bss` mis à zéro.
    ///
    /// C'est le chemin d'un binaire qu'on ouvre pour l'étudier. Il
    /// diffère de [`Elf::image`] sur trois points qui comptent :
    ///
    /// - il lit les en-têtes de programme, qui survivent à `strip`, là où
    ///   les sections disparaissent ;
    /// - il projette des **pages entières** : entre la fin d'une section
    ///   et le début de la suivante, un vrai processus n'a pas de trou,
    ///   et une lecture qui tombe dans le rembourrage ne doit pas être
    ///   refusée alors que la vraie machine l'accepte ;
    /// - il respecte `memsz`, donc réserve le `.bss` même quand le
    ///   fichier n'en porte pas un octet.
    ///
    /// Rend `None` pour un fichier sans segment chargeable — un `.o`,
    /// que `ld` n'a pas encore vu.
    pub fn load_image(&self) -> Option<(u64, Vec<u8>)> {
        let début = self
            .segments
            .iter()
            .map(|segment| page_down(segment.vaddr))
            .min()?;
        let fin = self
            .segments
            .iter()
            .map(|segment| page_up(segment.vaddr.saturating_add(segment.memsz as u64)))
            .max()?;
        let taille = fin.checked_sub(début)? as usize;
        let mut image = vec![0u8; taille];
        for segment in &self.segments {
            let at = (segment.vaddr - début) as usize;
            let depuis = segment.offset;
            let combien = segment.filesz.min(image.len().saturating_sub(at));
            if let Some(source) = self.bytes_at(depuis, combien) {
                image[at..at + source.len()].copy_from_slice(source);
            }
        }
        Some((début, image))
    }

    /// Les octets du fichier, quand on les a encore.
    ///
    /// `load_image` est appelée sur un `Elf` qui ne porte pas le fichier :
    /// c'est l'appelant qui le garde. Le détour par une fermeture serait
    /// plus élégant et beaucoup moins lisible ; le champ `file` l'est.
    fn bytes_at(&self, offset: usize, len: usize) -> Option<&[u8]> {
        self.file.get(offset..offset.checked_add(len)?)
    }

    /// Applique les relocations, comme le ferait `ld.so` avant la
    /// première instruction.
    ///
    /// Sans elles, un binaire moderne démarre sur des tables de pointeurs
    /// entièrement nulles : le moindre appel indirect saute à zéro, et le
    /// message parle d'une adresse non projetée là où le vrai problème
    /// est qu'un chargeur n'est pas passé.
    ///
    /// `bias` est ce que le chargeur a ajouté aux adresses du fichier —
    /// nul ici, puisque namsdec pose un binaire à *ses* adresses pour que
    /// ce qu'on lit à l'écran soit ce qu'écrivent `objdump` et les
    /// autres. Il est tout de même respecté : le jour où une base est
    /// choisie, le relogement suit sans être réécrit.
    ///
    /// `absent` décide de ce qu'on met dans une case qui attend une
    /// fonction vivant dans une bibliothèque : namsdec n'en charge
    /// aucune, et la seule réponse honnête est une adresse qui, si on
    /// l'appelle, sait dire de qui il s'agissait.
    pub fn relocate(
        &self,
        image_addr: u64,
        image: &mut [u8],
        bias: u64,
        mut absent: impl FnMut(&Reloc) -> Option<u64>,
    ) -> RelocReport {
        let mut report = RelocReport::default();
        // La taille du bloc TLS du programme principal : les décalages
        // `TPOFF` se comptent *en dessous* du pointeur de fil.
        let tls_size = self
            .tls
            .map(|tls| align_up(tls.memsz as u64, tls.align))
            .unwrap_or(0);

        for reloc in &self.relocs {
            let at = reloc.at.wrapping_add(bias);
            let valeur = match reloc.kind {
                RelocKind::Relative => {
                    if reloc.implicit {
                        // `DT_RELR` : la case porte déjà l'adresse, il n'y
                        // a qu'à la décaler.
                        read_u64(image, image_addr, at).map(|déjà| déjà.wrapping_add(bias))
                    } else {
                        Some(bias.wrapping_add(reloc.addend as u64))
                    }
                }
                RelocKind::GlobDat | RelocKind::JumpSlot | RelocKind::Direct64 => {
                    if reloc.external() {
                        match absent(reloc) {
                            Some(adresse) => {
                                report.external.push(reloc.symbol.clone());
                                Some(adresse)
                            }
                            None => None,
                        }
                    } else {
                        Some(
                            bias.wrapping_add(reloc.symbol_value)
                                .wrapping_add(reloc.addend as u64),
                        )
                    }
                }
                // Une fonction choisie à l'exécution : il faudrait
                // *appeler* le résolveur, ce qu'un chargeur fait et que
                // namsdec ne fera pas dans le dos de celui qui regarde.
                RelocKind::Irelative => {
                    report.ifunc += 1;
                    absent(reloc)
                }
                RelocKind::TlsOffset => {
                    // Le décalage depuis le pointeur de fil : négatif,
                    // puisque le bloc TLS du programme principal est
                    // posé juste avant lui.
                    Some(
                        (reloc.symbol_value.wrapping_add(reloc.addend as u64))
                            .wrapping_sub(tls_size),
                    )
                }
                // Copier une donnée depuis une bibliothèque qui n'est pas
                // là n'a pas de sens : la case garde ce que le fichier y
                // a mis, et le rapport le dit.
                RelocKind::Copy => {
                    report.skipped.push(reloc.kind);
                    None
                }
                RelocKind::Other(_) => {
                    report.skipped.push(reloc.kind);
                    None
                }
            };
            let Some(valeur) = valeur else { continue };
            if write_u64(image, image_addr, at, valeur) {
                report.applied += 1;
            } else {
                report.outside += 1;
            }
        }
        report.external.sort();
        report.external.dedup();
        report
    }

    /// Aplatit les sections en une image mémoire contiguë : l'adresse de
    /// départ, et les octets à y charger.
    ///
    /// Les trous entre sections (l'écart entre `.text` et `.data`, aligné
    /// sur une page) sont comblés par des zéros — comme `.bss`, qui n'a
    /// rien à copier mais doit exister en mémoire.
    pub fn image(&self) -> Option<(u64, Vec<u8>)> {
        let start = self.sections.iter().map(|s| s.addr).min()?;
        let end = self.sections.iter().map(|s| s.addr + s.size as u64).max()?;
        let mut image = vec![0u8; (end - start) as usize];
        for section in &self.sections {
            let at = (section.addr - start) as usize;
            image[at..at + section.data.len()].copy_from_slice(&section.data);
        }
        Some((start, image))
    }
}

#[cfg(test)]
mod tests {

    /// Les étiquettes du source se retrouvent dans l'ELF lié, à leur
    /// adresse — c'est ce qui permet de dire « tampon » plutôt que
    /// « 0x401000 ».
    #[test]
    fn labels_come_back_with_their_addresses() {
        let Some(_) = crate::find_nasm() else {
            eprintln!("nasm absent : test ignoré");
            return;
        };
        if !crate::find_linker() {
            eprintln!("ld absent : test ignoré");
            return;
        }
        let source = "\
global _start
section .data
tampon: db 1, 2, 3, 4
section .text
_start:
    mov al, [tampon]
    hlt
";
        let out = crate::assemble(source, 0x400000).expect("le programme doit s'assembler");
        let tampon = out
            .symbols
            .iter()
            .find(|s| s.name == "tampon")
            .expect("l'étiquette tampon doit être connue");
        let start = out
            .symbols
            .iter()
            .find(|s| s.name == "_start")
            .expect("le point d'entrée aussi");
        assert_eq!(start.addr, out.entry);
        assert!(
            tampon.addr != start.addr,
            "les deux étiquettes sont dans des sections différentes"
        );
        // Triés par adresse : l'interface s'appuie dessus pour trouver
        // « la dernière étiquette avant cette adresse ».
        let mut adresses: Vec<u64> = out.symbols.iter().map(|s| s.addr).collect();
        let triees = adresses.clone();
        adresses.sort_unstable();
        assert_eq!(adresses, triees, "les symboles doivent être triés");
        // Et pas de bruit d'assembleur dans la liste.
        assert!(
            !out.symbols.iter().any(|s| s.name.ends_with(".asm")),
            "le nom du fichier n'est pas une étiquette : {:?}",
            out.symbols
        );
    }
}


#[cfg(test)]
mod loader_tests {
    use super::*;

    /// Un binaire du système se lit par ses **segments**, et non par ses
    /// sections.
    ///
    /// C'est la lecture du noyau, et la seule qui survive à `strip`. Elle
    /// donne une image en pages entières, sans trou entre deux sections —
    /// là où la lecture par sections laissait le rembourrage hors de la
    /// mémoire projetée.
    #[test]
    fn a_real_binary_is_read_by_its_segments() {
        let Ok(octets) = std::fs::read("/bin/ls") else {
            return;
        };
        let elf = parse(&octets).expect("un ELF64");

        assert!(
            !elf.segments.is_empty(),
            "un exécutable déclare ce qu'il faut charger"
        );
        assert!(
            elf.segments.iter().any(|segment| segment.execute),
            "dont au moins un segment de code"
        );
        // Le point d'entrée est dans un segment exécutable : c'est la
        // définition même d'un exécutable qui démarre.
        assert!(
            elf.segments.iter().any(|segment| segment.execute
                && elf.entry >= segment.vaddr
                && elf.entry < segment.vaddr + segment.memsz as u64),
            "le point d'entrée {:#x} doit être dans un segment exécutable",
            elf.entry
        );

        let (début, image) = elf.load_image().expect("une image");
        assert_eq!(début % PAGE, 0, "l'image commence sur une page");
        assert_eq!(image.len() as u64 % PAGE, 0, "et finit sur une page");
        // Elle couvre le point d'entrée, et ce qu'on y lit est bien le
        // code du fichier.
        let offset = (elf.entry - début) as usize;
        assert!(offset + 16 < image.len(), "l'image couvre le point d'entrée");
        // La table des en-têtes est projetée : c'est par elle que le
        // programme retrouve son propre modèle TLS.
        assert!(elf.phdr.is_some(), "les en-têtes de programme sont projetés");
    }

    /// Les relocations sont lues, et appliquées.
    ///
    /// Un exécutable indépendant de la position range presque tous ses
    /// pointeurs dans des relocations relatives : chargé sans elles, il
    /// démarre sur des tables entièrement nulles.
    #[test]
    fn relocations_are_read_and_applied() {
        let Ok(octets) = std::fs::read("/bin/ls") else {
            return;
        };
        let elf = parse(&octets).expect("un ELF64");
        if elf.relocs.is_empty() {
            // Un exécutable à position fixe n'en a pas : rien à vérifier.
            return;
        }

        let (début, mut image) = elf.load_image().expect("une image");
        let rapport = elf.relocate(début, &mut image, 0, |_| Some(0xdead_0000));
        assert!(
            rapport.applied > 0,
            "des cases doivent avoir été remplies : {rapport:?}"
        );
        assert_eq!(rapport.outside, 0, "aucune ne vise hors de l'image");

        // Une relocation relative écrit une adresse qui tombe dans le
        // programme lui-même — c'est tout son objet.
        let fin = début + image.len() as u64;
        let relative = elf
            .relocs
            .iter()
            .find(|reloc| reloc.kind == RelocKind::Relative && !reloc.implicit);
        if let Some(reloc) = relative {
            let offset = (reloc.at - début) as usize;
            let écrit = u64::from_le_bytes(image[offset..offset + 8].try_into().unwrap());
            assert!(
                (début..fin).contains(&écrit),
                "une relocation relative pointe dans le programme : {écrit:#x}"
            );
        }

        // Et ce qui viendrait d'une bibliothèque est nommé, pas deviné.
        for reloc in elf.relocs.iter().filter(|reloc| reloc.external()) {
            assert!(!reloc.symbol.is_empty(), "un symbole externe porte un nom");
        }
    }

    /// Un `.o` n'est pas chargeable, et le dit.
    #[test]
    fn an_unlinked_object_has_nothing_to_load() {
        let Some(_) = crate::find_nasm() else {
            return;
        };
        let dir = tempfile::tempdir().expect("un dossier");
        let source = dir.path().join("o.asm");
        std::fs::write(&source, "section .text\nglobal _start\n_start:\n hlt\n").unwrap();
        let objet = dir.path().join("o.o");
        let fait = std::process::Command::new("nasm")
            .args(["-f", "elf64", "-o"])
            .arg(&objet)
            .arg(&source)
            .status();
        if !fait.is_ok_and(|s| s.success()) {
            return;
        }
        let elf = parse(&std::fs::read(&objet).unwrap()).expect("c'est bien un ELF64");
        assert_eq!(elf.kind, Kind::Relocatable, "`ld` n'est pas passé");
        assert!(elf.segments.is_empty(), "rien à charger");
        assert!(elf.load_image().is_none(), "et donc pas d'image");
    }
}
