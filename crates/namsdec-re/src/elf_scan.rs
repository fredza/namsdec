//! Ce qu'un ELF64 dit de lui-même : comment il sera chargé, et ce qui le
//! protège.
//!
//! C'est la lecture qu'on fait d'un binaire qu'on n'a pas écrit — celle
//! de `checksec`, `readelf -l`, `file`. Elle ne charge rien et n'exécute
//! rien : elle relit les en-têtes, les segments, la table dynamique et
//! les symboles, et en tire ce qui change la façon de l'attaquer ou de
//! le défendre.
//!
//! Distincte de `namsdec_asm::elf`, qui lit le strict nécessaire pour
//! charger *notre* sortie de `ld` en mémoire simulée. Ici les exigences
//! sont inverses : on suppose un fichier hostile ou inconnu, rien n'est
//! obligatoire, et chaque absence est elle-même une information.

/// Ce que le fichier est.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Un objet à lier (`.o`).
    Relocatable,
    /// Un exécutable à adresse fixe.
    Executable,
    /// Position-indépendant : exécutable PIE ou bibliothèque partagée.
    Shared,
    Core,
    Other(u16),
}

/// L'état de la zone de données relogeables après le démarrage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relro {
    /// Rien n'est repassé en lecture seule.
    None,
    /// La zone existe, mais la table des sauts (`.got.plt`) reste
    /// inscriptible : une écriture arbitraire peut encore la détourner.
    Partial,
    /// Toutes les relocations sont résolues au démarrage, puis la zone
    /// passe en lecture seule.
    Full,
}

/// Un segment tel que le noyau le projettera.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub kind: &'static str,
    /// Où le segment commence dans le fichier : c'est par là qu'on
    /// retrouve dans les octets ce que le programme verra à `vaddr`.
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Segment {
    /// Les trois droits d'un coup : `r-x`, `rw-`.
    pub fn rights(&self) -> String {
        format!(
            "{}{}{}",
            if self.read { 'r' } else { '-' },
            if self.write { 'w' } else { '-' },
            if self.execute { 'x' } else { '-' }
        )
    }
}

/// Une section, telle que l'éditeur de liens l'a rangée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionInfo {
    pub name: String,
    pub addr: u64,
    pub size: u64,
    pub write: bool,
    pub execute: bool,
    /// Où la section commence dans le fichier.
    ///
    /// Sans lui, on ne peut pas aller lire son contenu : une adresse
    /// virtuelle ne dit pas où chercher dans les octets qu'on tient.
    pub offset: u64,
    /// La section n'occupe rien dans le fichier — `.bss` et ses
    /// semblables. Son `offset` désigne alors ce qui suit, et lire là
    /// reviendrait à raconter le contenu du voisin.
    pub nobits: bool,
}

/// Une fonction nommée par la table des symboles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub addr: u64,
    pub size: u64,
}

/// Les défenses que le processeur lui-même applique, quand le binaire
/// les réclame — le fichier le dit dans une note que le noyau relit au
/// chargement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cet {
    /// Les sauts indirects doivent atterrir sur un `endbr64`.
    pub ibt: bool,
    /// Le processeur tient sa propre pile d'adresses de retour et
    /// compare.
    pub shadow_stack: bool,
}

/// Tout ce que l'en-tête et les tables racontent du binaire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scan {
    pub class64: bool,
    pub little_endian: bool,
    pub kind: Kind,
    pub machine: &'static str,
    pub entry: u64,
    /// L'interpréteur réclamé (`/lib64/ld-linux-x86-64.so.2`), s'il y en
    /// a un. Son absence veut dire « lié statiquement ».
    pub interpreter: Option<String>,
    /// Le segment de pile est-il non exécutable ? `None` quand le
    /// fichier n'en déclare pas — le noyau décide alors seul, et
    /// historiquement il l'a rendue exécutable.
    pub nx: Option<bool>,
    pub relro: Relro,
    /// Le programme vérifie-t-il un canari de pile au retour de ses
    /// fonctions ?
    pub canary: bool,
    /// Les fonctions à bornes vérifiées (`__memcpy_chk`…) que le
    /// compilateur a mises à la place des originales.
    pub fortified: Vec<String>,
    /// Aucune table de symboles : le binaire a été dépouillé.
    pub stripped: bool,
    /// Les bibliothèques réclamées au chargement.
    pub needed: Vec<String>,
    pub runpath: Option<String>,
    pub rpath: Option<String>,
    pub segments: Vec<Segment>,
    pub sections: Vec<SectionInfo>,
    /// Les fonctions que la table des symboles nomme, par adresse.
    pub functions: Vec<Function>,
    /// Le nombre de symboles lisibles, toutes tables confondues.
    pub symbol_count: usize,
    pub cet: Cet,
    /// L'empreinte que l'éditeur de liens a posée : deux binaires ayant
    /// le même sont le même.
    pub build_id: Option<String>,
}

impl Scan {
    /// Le binaire est-il chargé à une adresse imposée par lui ?
    ///
    /// Un exécutable `DYN` est relogeable : le noyau le pose où il veut,
    /// et les adresses changent à chaque exécution (ASLR). Un `EXEC` a
    /// ses adresses écrites en dur — celles qu'on lit dans le
    /// désassemblage sont celles qu'il aura.
    pub fn pie(&self) -> bool {
        self.kind == Kind::Shared
    }

    /// Un segment à la fois inscriptible et exécutable : de quoi écrire
    /// du code puis sauter dedans, sans avoir à contourner quoi que ce
    /// soit.
    pub fn writable_executable(&self) -> Vec<&Segment> {
        self.segments
            .iter()
            .filter(|s| s.write && s.execute)
            .collect()
    }

    pub fn dynamic(&self) -> bool {
        self.interpreter.is_some()
    }
}

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;
const PT_NOTE: u32 = 4;
const PT_PHDR: u32 = 6;
const PT_TLS: u32 = 7;
const PT_GNU_EH_FRAME: u32 = 0x6474_e550;
const PT_GNU_STACK: u32 = 0x6474_e551;
const PT_GNU_RELRO: u32 = 0x6474_e552;
const PT_GNU_PROPERTY: u32 = 0x6474_e553;

const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const DT_BIND_NOW: u64 = 24;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;
const DT_FLAGS: u64 = 30;
const DT_FLAGS_1: u64 = 0x6fff_fffb;
const DF_BIND_NOW: u64 = 0x8;
const DF_1_NOW: u64 = 0x1;

const SHT_SYMTAB: u32 = 2;
const SHT_NOTE: u32 = 7;
const STT_FUNC: u8 = 2;
const NT_GNU_BUILD_ID: u32 = 3;
const NT_GNU_PROPERTY_TYPE_0: u32 = 5;
const GNU_PROPERTY_X86_FEATURE_1_AND: u32 = 0xc000_0002;
const X86_FEATURE_1_IBT: u32 = 0x1;
const X86_FEATURE_1_SHSTK: u32 = 0x2;
const SHT_DYNSYM: u32 = 11;
/// Une section qui n'occupe rien dans le fichier : `.bss`.
const SHT_NOBITS: u32 = 8;

/// Les trois lecteurs, et la seule chose qu'ils promettent : **rien ne
/// panique**.
///
/// `at` vient toujours, de près ou de loin, d'un champ du fichier. Sur un
/// ELF hostile il est quelconque, et `at + 4` déborde — ce qui, en debug,
/// panique au lieu de rendre un index faux. Une position qui ne tient pas
/// dans un `usize` n'est pas une position : il n'y a rien à cette
/// adresse, et `None` est la réponse exacte.
fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let fin = at.checked_add(2)?;
    Some(u16::from_le_bytes(bytes.get(at..fin)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let fin = at.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(at..fin)?.try_into().ok()?))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    let fin = at.checked_add(8)?;
    Some(u64::from_le_bytes(bytes.get(at..fin)?.try_into().ok()?))
}

/// La chaîne terminée par zéro qui commence à `at`.
fn string_at(bytes: &[u8], at: usize) -> Option<String> {
    let rest = bytes.get(at..)?;
    let end = rest.iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&rest[..end]).into_owned())
}

fn segment_kind(kind: u32) -> &'static str {
    match kind {
        PT_LOAD => "LOAD",
        PT_DYNAMIC => "DYNAMIC",
        PT_INTERP => "INTERP",
        PT_NOTE => "NOTE",
        PT_PHDR => "PHDR",
        PT_TLS => "TLS",
        PT_GNU_EH_FRAME => "GNU_EH_FRAME",
        PT_GNU_STACK => "GNU_STACK",
        PT_GNU_RELRO => "GNU_RELRO",
        PT_GNU_PROPERTY => "GNU_PROPERTY",
        _ => "?",
    }
}

fn machine_name(machine: u16) -> &'static str {
    match machine {
        0x3e => "x86-64",
        0x03 => "x86 (32 bits)",
        0xb7 => "AArch64",
        0x28 => "ARM",
        0xf3 => "RISC-V",
        _ => "?",
    }
}

/// Lit le binaire. Rend `None` si ce n'est pas un ELF64 petit-boutiste —
/// le seul format que namsdec exécute.
pub fn scan(bytes: &[u8]) -> Option<Scan> {
    if bytes.get(..4)? != b"\x7fELF" {
        return None;
    }
    let class64 = *bytes.get(4)? == 2;
    let little_endian = *bytes.get(5)? == 1;
    if !class64 || !little_endian {
        return None;
    }

    let kind = match u16_at(bytes, 0x10)? {
        1 => Kind::Relocatable,
        2 => Kind::Executable,
        3 => Kind::Shared,
        4 => Kind::Core,
        other => Kind::Other(other),
    };
    let machine = machine_name(u16_at(bytes, 0x12)?);
    let entry = u64_at(bytes, 0x18)?;

    let phoff = u64_at(bytes, 0x20)? as usize;
    let phentsize = u16_at(bytes, 0x36)? as usize;
    let phnum = u16_at(bytes, 0x38)? as usize;

    let mut segments = Vec::new();
    let mut interpreter = None;
    let mut nx = None;
    let mut relro = Relro::None;
    let mut dynamic_at = None;
    for index in 0..phnum {
        let header = phoff.checked_add(index.checked_mul(phentsize)?)?;
        let Some(kind) = u32_at(bytes, header) else {
            break;
        };
        let flags = u32_at(bytes, header.saturating_add(0x04))?;
        let offset = u64_at(bytes, header.saturating_add(0x08))?;
        let vaddr = u64_at(bytes, header.saturating_add(0x10))?;
        let filesz = u64_at(bytes, header.saturating_add(0x20))?;
        let memsz = u64_at(bytes, header.saturating_add(0x28))?;
        match kind {
            PT_INTERP => {
                interpreter = string_at(bytes, offset as usize);
            }
            // Les droits du segment de pile *sont* la réponse : le noyau
            // donne à la pile ceux que le fichier demande ici.
            PT_GNU_STACK => nx = Some(flags & 1 == 0),
            PT_GNU_RELRO => relro = Relro::Partial,
            PT_DYNAMIC => dynamic_at = Some((offset as usize, filesz as usize)),
            _ => {}
        }
        segments.push(Segment {
            kind: segment_kind(kind),
            offset,
            vaddr,
            filesz,
            memsz,
            read: flags & 4 != 0,
            write: flags & 2 != 0,
            execute: flags & 1 != 0,
        });
    }

    let mut needed = Vec::new();
    let mut runpath = None;
    let mut rpath = None;
    if let Some((offset, size)) = dynamic_at {
        let dyn_info = read_dynamic(bytes, offset, size, &segments);
        needed = dyn_info.needed;
        runpath = dyn_info.runpath;
        rpath = dyn_info.rpath;
        // La zone relogeable ne sert vraiment qu'une fois toutes les
        // relocations faites au démarrage : sinon la table des sauts
        // reste inscriptible, et c'est elle qu'on détourne.
        if relro == Relro::Partial && dyn_info.bind_now {
            relro = Relro::Full;
        }
    }

    let (symbols, functions) = read_symbols(bytes).unwrap_or_default();
    let canary = symbols.iter().any(|name| name == "__stack_chk_fail");
    let mut fortified: Vec<String> = symbols
        .iter()
        .filter(|name| name.ends_with("_chk") && *name != "__stack_chk_fail")
        .cloned()
        .collect();
    fortified.sort();
    fortified.dedup();

    Some(Scan {
        class64,
        little_endian,
        kind,
        machine,
        entry,
        interpreter,
        nx,
        relro,
        canary,
        fortified,
        stripped: !has_symtab(bytes).unwrap_or(false),
        needed,
        runpath,
        rpath,
        segments,
        sections: read_sections(bytes).unwrap_or_default(),
        functions,
        symbol_count: symbols.len(),
        cet: read_cet(bytes).unwrap_or_default(),
        build_id: read_build_id(bytes),
    })
}

#[derive(Default)]
struct Dynamic {
    needed: Vec<String>,
    runpath: Option<String>,
    rpath: Option<String>,
    bind_now: bool,
}

/// La table dynamique : ce que le chargeur devra faire avant de rendre la
/// main au programme.
fn read_dynamic(bytes: &[u8], offset: usize, size: usize, segments: &[Segment]) -> Dynamic {
    let mut info = Dynamic::default();
    let mut strtab = None;
    let mut wanted: Vec<(u64, u64)> = Vec::new();
    let mut at = offset;
    // Bornes saturantes : `offset` et `size` sont des champs du fichier,
    // et leur somme déborde sur une table hostile. Saturer les mène hors
    // du fichier, où les lecteurs rendent `None` — ce qui est la réponse
    // juste.
    let fin_table = offset.saturating_add(size);
    while at.saturating_add(16) <= fin_table {
        let Some(tag) = u64_at(bytes, at) else { break };
        let Some(value) = u64_at(bytes, at.saturating_add(8)) else {
            break;
        };
        at = at.saturating_add(16);
        if tag == DT_NULL {
            break;
        }
        match tag {
            DT_STRTAB => strtab = Some(value),
            DT_BIND_NOW => info.bind_now = true,
            DT_FLAGS if value & DF_BIND_NOW != 0 => info.bind_now = true,
            DT_FLAGS_1 if value & DF_1_NOW != 0 => info.bind_now = true,
            DT_NEEDED | DT_RUNPATH | DT_RPATH => wanted.push((tag, value)),
            _ => {}
        }
    }
    // Les noms sont des décalages dans une table qu'on ne connaît que
    // par son adresse virtuelle : il faut la retrouver dans le fichier.
    let Some(strtab) = strtab.and_then(|addr| file_offset(addr, segments)) else {
        return info;
    };
    for (tag, value) in wanted {
        // `strtab` vient des segments, `value` de la table dynamique :
        // l'addition déborde sur un fichier qui annonce n'importe quoi.
        let Some(text) = strtab
            .checked_add(value as usize)
            .and_then(|at| string_at(bytes, at))
        else {
            continue;
        };
        match tag {
            DT_NEEDED => info.needed.push(text),
            DT_RUNPATH => info.runpath = Some(text),
            DT_RPATH => info.rpath = Some(text),
            _ => {}
        }
    }
    info
}

/// L'adresse virtuelle `addr`, ramenée à sa place dans le fichier.
fn file_offset(addr: u64, segments: &[Segment]) -> Option<usize> {
    // Les segments gardent leur `vaddr` et leur taille ; le décalage
    // fichier se retrouve en cherchant le LOAD qui contient l'adresse.
    // On refait le calcul à partir de `p_offset`, conservé implicitement
    // par la différence vaddr - offset, constante dans un LOAD.
    segments
        .iter()
        .find(|s| s.kind == "LOAD" && addr >= s.vaddr && addr < s.vaddr + s.filesz)
        .map(|s| (addr - s.vaddr + s.offset) as usize)
}

/// Les noms de toutes les tables de symboles présentes — `.symtab` quand
/// le binaire n'est pas dépouillé, `.dynsym` toujours pour un binaire
/// dynamique.
#[allow(clippy::type_complexity)]
fn read_symbols(bytes: &[u8]) -> Option<(Vec<String>, Vec<Function>)> {
    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    let mut names = Vec::new();
    let mut functions: Vec<Function> = Vec::new();
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        let kind = u32_at(bytes, header.saturating_add(0x04))?;
        if kind != SHT_SYMTAB && kind != SHT_DYNSYM {
            continue;
        }
        let strtab = u32_at(bytes, header.saturating_add(0x28))? as usize;
        let strtab_header = shoff.checked_add(strtab.checked_mul(shentsize)?)?;
        let str_offset = u64_at(bytes, strtab_header.saturating_add(0x18))? as usize;
        let offset = u64_at(bytes, header.saturating_add(0x18))? as usize;
        let size = u64_at(bytes, header.saturating_add(0x20))? as usize;
        let entry_size = u64_at(bytes, header.saturating_add(0x38))? as usize;
        if entry_size == 0 {
            continue;
        }
        for at in (offset..offset.checked_add(size)?).step_by(entry_size) {
            let Some(name_offset) = u32_at(bytes, at) else {
                break;
            };
            let Some(name) = str_offset
                .checked_add(name_offset as usize)
                .and_then(|at| string_at(bytes, at))
            else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            // Le type est dans les quatre bits bas de st_info ; une
            // fonction déclarée porte une adresse et souvent une taille.
            let info = bytes.get(at.saturating_add(4)).copied().unwrap_or(0);
            let value = u64_at(bytes, at.saturating_add(8)).unwrap_or(0);
            let size = u64_at(bytes, at.saturating_add(16)).unwrap_or(0);
            if info & 0xf == STT_FUNC && value != 0 {
                functions.push(Function {
                    name: name.clone(),
                    addr: value,
                    size,
                });
            }
            names.push(name);
        }
    }
    functions.sort_by_key(|f| (f.addr, f.name.clone()));
    functions.dedup();
    Some((names, functions))
}

/// Les sections, pour qui veut savoir où le code s'arrête et où les
/// données commencent — un découpage plus fin que celui des segments.
fn read_sections(bytes: &[u8]) -> Option<Vec<SectionInfo>> {
    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    let shstrndx = u16_at(bytes, 0x3e)? as usize;
    let shstr_header = shoff.checked_add(shstrndx.checked_mul(shentsize)?)?;
    let names_at = u64_at(bytes, shstr_header.saturating_add(0x18))? as usize;

    let mut sections = Vec::new();
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        let name_offset = u32_at(bytes, header)? as usize;
        let flags = u64_at(bytes, header.saturating_add(0x08))?;
        let kind = u32_at(bytes, header.saturating_add(0x04))?;
        let addr = u64_at(bytes, header.saturating_add(0x10))?;
        let offset = u64_at(bytes, header.saturating_add(0x18))?;
        let size = u64_at(bytes, header.saturating_add(0x20))?;
        let name = string_at(bytes, names_at + name_offset)?;
        if name.is_empty() {
            continue;
        }
        sections.push(SectionInfo {
            name,
            addr,
            size,
            write: flags & 0x1 != 0,
            execute: flags & 0x4 != 0,
            offset,
            nobits: kind == SHT_NOBITS,
        });
    }
    Some(sections)
}

/// Les notes du fichier : chacune porte un nom, un type, et des octets.
///
/// Rend les triplets (nom, type, contenu) de toutes les sections de
/// notes — c'est là que vivent l'empreinte de build et les drapeaux que
/// le processeur applique.
fn notes(bytes: &[u8]) -> Vec<(String, u32, Vec<u8>)> {
    let mut found = Vec::new();
    let Some(shoff) = u64_at(bytes, 0x28).map(|v| v as usize) else {
        return found;
    };
    let (Some(shentsize), Some(shnum)) = (
        u16_at(bytes, 0x3a).map(|v| v as usize),
        u16_at(bytes, 0x3c).map(|v| v as usize),
    ) else {
        return found;
    };
    for index in 0..shnum {
        let Some(header) = shoff.checked_add(index * shentsize) else {
            break;
        };
        if u32_at(bytes, header.saturating_add(0x04)) != Some(SHT_NOTE) {
            continue;
        }
        let (Some(offset), Some(size)) = (
            u64_at(bytes, header.saturating_add(0x18)).map(|v| v as usize),
            u64_at(bytes, header.saturating_add(0x20)).map(|v| v as usize),
        ) else {
            continue;
        };
        let mut at = offset;
        let fin_notes = offset.saturating_add(size);
        while at.saturating_add(12) <= fin_notes {
            let (Some(name_size), Some(desc_size), Some(kind)) = (
                u32_at(bytes, at).map(|v| v as usize),
                u32_at(bytes, at.saturating_add(4)).map(|v| v as usize),
                u32_at(bytes, at.saturating_add(8)),
            ) else {
                break;
            };
            // Nom et contenu sont alignés sur quatre octets chacun. Les
            // additions saturent : les tailles viennent du fichier, et
            // une note hostile les annonce énormes.
            let name_at = at.saturating_add(12);
            let desc_at = name_at.saturating_add(name_size.next_multiple_of(4));
            let end = desc_at.saturating_add(desc_size.next_multiple_of(4));
            let name = string_at(bytes, name_at).unwrap_or_default();
            if let Some(desc) = desc_at
                .checked_add(desc_size)
                .and_then(|fin| bytes.get(desc_at..fin))
            {
                found.push((name, kind, desc.to_vec()));
            }
            if end <= at {
                break;
            }
            at = end;
        }
    }
    found
}

/// Les défenses matérielles réclamées dans `.note.gnu.property`.
fn read_cet(bytes: &[u8]) -> Option<Cet> {
    let mut cet = Cet::default();
    for (name, kind, desc) in notes(bytes) {
        if name != "GNU" || kind != NT_GNU_PROPERTY_TYPE_0 {
            continue;
        }
        // Une suite de propriétés : type, longueur, valeur, le tout
        // aligné sur huit octets en 64 bits.
        let mut at = 0usize;
        while at.saturating_add(8) <= desc.len() {
            let Some(kind) = u32_at(&desc, at) else { break };
            let Some(size) = u32_at(&desc, at.saturating_add(4)).map(|v| v as usize) else {
                break;
            };
            let value_at = at.saturating_add(8);
            if kind == GNU_PROPERTY_X86_FEATURE_1_AND
                && let Some(value) = u32_at(&desc, value_at)
            {
                cet.ibt |= value & X86_FEATURE_1_IBT != 0;
                cet.shadow_stack |= value & X86_FEATURE_1_SHSTK != 0;
            }
            at = value_at.saturating_add(size.next_multiple_of(8));
        }
    }
    Some(cet)
}

/// L'empreinte posée par l'éditeur de liens, en hexadécimal.
fn read_build_id(bytes: &[u8]) -> Option<String> {
    notes(bytes)
        .into_iter()
        .find(|(name, kind, _)| name == "GNU" && *kind == NT_GNU_BUILD_ID)
        .map(|(_, _, desc)| desc.iter().map(|b| format!("{b:02x}")).collect())
}

fn has_symtab(bytes: &[u8]) -> Option<bool> {
    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        if u32_at(bytes, header.saturating_add(0x04))? == SHT_SYMTAB {
            return Some(true);
        }
    }
    Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'ELF que `ld` produit pour un programme d'apprentissage : pas de
    /// PIE, pas de bibliothèque, pas de canari — et c'est exactement ce
    /// qu'on veut lire à l'écran plutôt que de le supposer.
    #[test]
    fn a_hand_written_program_shows_what_it_has_and_what_it_lacks() {
        let Ok(elf) = build("global _start\nsection .text\n_start:\n    hlt\n") else {
            eprintln!("nasm ou ld absent : test ignoré");
            return;
        };
        let scan = scan(&elf).expect("un ELF64 produit par ld");
        assert_eq!(scan.kind, Kind::Executable);
        assert_eq!(scan.machine, "x86-64");
        assert!(
            !scan.pie(),
            "ld sans -pie donne un exécutable à adresse fixe"
        );
        assert!(!scan.dynamic(), "rien à charger : pas d'interpréteur");
        assert!(scan.needed.is_empty());
        assert!(!scan.canary, "personne n'a posé de canari ici");
        assert!(!scan.stripped, "les étiquettes du source sont encore là");
        assert!(
            scan.segments.iter().any(|s| s.kind == "LOAD" && s.execute),
            "il faut bien un segment de code : {:?}",
            scan.segments
        );
    }

    /// Le système en a de vrais sous la main : `/bin/ls` est compilé avec
    /// tout ce qu'une distribution active aujourd'hui.
    #[test]
    fn a_system_binary_shows_its_hardening() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        let Some(scan) = scan(&bytes) else {
            eprintln!("/bin/ls n'est pas un ELF64 ici : test ignoré");
            return;
        };
        assert!(scan.pie(), "les distributions compilent /bin/ls en PIE");
        assert!(scan.dynamic(), "et le lient dynamiquement");
        assert!(
            scan.needed.iter().any(|lib| lib.starts_with("libc.")),
            "il réclame la libc : {:?}",
            scan.needed
        );
        assert_eq!(scan.nx, Some(true), "pile non exécutable");
        assert_eq!(scan.relro, Relro::Full, "RELRO complet");
        assert!(
            scan.writable_executable().is_empty(),
            "aucun segment W+X dans un binaire de distribution"
        );
    }

    /// La même exigence pour l'analyse entière : vingt binaires du
    /// système, lus sans broncher, et ce qu'ils disent doit se tenir.
    ///
    /// Un analyseur qui marche sur un exemple et panique sur le
    /// vingtième fichier venu n'est pas un analyseur.
    #[test]
    fn the_whole_scan_holds_on_whatever_this_machine_has() {
        let Ok(dir) = std::fs::read_dir("/usr/bin") else {
            eprintln!("/usr/bin illisible : test ignoré");
            return;
        };
        let mut lus = 0usize;
        for entry in dir.flatten() {
            if lus >= 20 {
                break;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if !meta.is_file() || meta.len() > 4 << 20 {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            let Some(scan) = scan(&bytes) else { continue };
            lus += 1;
            let quoi = entry.path();

            // Un ELF64 x86-64 : c'est ce que la machine exécute, et
            // c'est ce que le scan doit reconnaître.
            assert!(scan.little_endian, "{quoi:?}");
            assert!(scan.entry > 0 || scan.kind == Kind::Relocatable, "{quoi:?}");
            // Les segments et les sections tiennent dans le fichier ou
            // en mémoire, mais ne racontent pas n'importe quoi.
            for segment in &scan.segments {
                assert!(
                    segment.memsz >= segment.filesz,
                    "{quoi:?} : un segment plus gros dans le fichier qu'en mémoire"
                );
            }
            for section in &scan.sections {
                assert!(!section.name.is_empty(), "{quoi:?} : section sans nom");
            }
            // Une fonction nommée a une adresse.
            for function in &scan.functions {
                assert!(!function.name.is_empty(), "{quoi:?}");
                assert!(function.addr > 0, "{quoi:?} : {}", function.name);
            }
            // Un binaire dynamique réclame au moins une bibliothèque ;
            // un statique n'a pas d'interpréteur. L'inverse serait le
            // signe qu'on a mal lu la table dynamique.
            if scan.interpreter.is_some() {
                assert!(
                    !scan.needed.is_empty(),
                    "{quoi:?} : un interpréteur mais aucune bibliothèque"
                );
            }
        }
        assert!(lus > 0, "aucun binaire lu : le test n'a rien prouvé");
    }

    /// Un binaire de distribution moderne réclame les défenses du
    /// processeur et porte son empreinte de build ; le nôtre, non — et
    /// c'est une chose de plus qu'on apprend en comparant les deux.
    #[test]
    fn a_system_binary_asks_the_processor_for_help() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        let Some(scan) = scan(&bytes) else {
            eprintln!("/bin/ls n'est pas un ELF64 ici : test ignoré");
            return;
        };
        assert!(
            scan.build_id.is_some(),
            "les distributions posent une empreinte de build"
        );
        assert!(
            scan.functions.iter().any(|f| f.name.contains("main"))
                || !scan.functions.is_empty()
                || scan.stripped,
            "soit des fonctions nommées, soit un binaire dépouillé"
        );
        assert!(
            scan.sections.iter().any(|s| s.name == ".text" && s.execute),
            "il y a bien une section de code : {:?}",
            scan.sections.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        // IBT/SHSTK ne sont pas partout, mais quand ils sont là, ils
        // viennent de la note, pas d'une supposition.
        if scan.cet.ibt {
            assert!(
                scan.sections.iter().any(|s| s.name == ".note.gnu.property"),
                "IBT se lit dans .note.gnu.property"
            );
        }
    }

    /// Ce qui n'est pas un ELF64 est refusé net, plutôt que lu de
    /// travers : un binaire brut désassemblé n'a pas d'en-tête du tout.
    #[test]
    fn anything_else_is_refused() {
        assert!(scan(b"").is_none());
        assert!(scan(b"MZ\x90\x00").is_none(), "un exécutable Windows");
        // Un ELF32 : bon nombre magique, mauvaise classe.
        let mut elf32 = b"\x7fELF".to_vec();
        elf32.push(1);
        elf32.extend(std::iter::repeat_n(0u8, 60));
        assert!(scan(&elf32).is_none());
    }

    /// Assemble et lie un programme, et rend le fichier ELF.
    fn build(source: &str) -> Result<Vec<u8>, ()> {
        use std::process::Command;
        let dir = std::env::temp_dir().join(format!("namsdec-scan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|_| ())?;
        let asm = dir.join("p.asm");
        let obj = dir.join("p.o");
        let bin = dir.join("p");
        std::fs::write(&asm, source).map_err(|_| ())?;
        let nasm = Command::new("nasm")
            .args(["-f", "elf64", "-o"])
            .arg(&obj)
            .arg(&asm)
            .status()
            .map_err(|_| ())?;
        if !nasm.success() {
            return Err(());
        }
        let ld = Command::new("ld")
            .arg("-o")
            .arg(&bin)
            .arg(&obj)
            .status()
            .map_err(|_| ())?;
        if !ld.success() {
            return Err(());
        }
        let bytes = std::fs::read(&bin).map_err(|_| ())?;
        let _ = std::fs::remove_dir_all(&dir);
        Ok(bytes)
    }
}
