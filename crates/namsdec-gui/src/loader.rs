//! Le chargeur : tout ce qui se passe entre « ouvrir un fichier » et la
//! première instruction.
//!
//! Un exécutable ne se contente pas d'être posé en mémoire. Avant que
//! `_start` reçoive la main, le noyau et l'éditeur de liens dynamique ont
//! déjà fait un travail considérable, et un programme moderne compte
//! dessus au point de ne pas survivre trois instructions sans lui :
//!
//! 1. les **segments** sont projetés, en pages entières, avec leurs
//!    droits — et le `.bss` mis à zéro ;
//! 2. les **relocations** sont appliquées : sans elles, les tables de
//!    pointeurs d'un binaire indépendant de la position sont nulles ;
//! 3. un bloc de **mémoire locale au fil d'exécution** (TLS) est posé, et
//!    la base `FS` installée dessus — c'est là que vit le canari de pile ;
//! 4. la **pile initiale** est écrite : `argc`, `argv`, l'environnement,
//!    puis le vecteur auxiliaire par lequel le programme apprend où sont
//!    ses propres en-têtes et d'où vient son hasard.
//!
//! namsdec faisait la première étape, à moitié, et aucune des trois
//! autres. Un binaire ouvert démarrait donc sur une pile de zéros, sans
//! TLS et sans relocations : il n'allait pas loin, et rien ne disait
//! pourquoi.
//!
//! Ce que ce module ne fait **pas**, et ne fera pas en cachette : charger
//! une bibliothèque, résoudre `printf`, appeler un résolveur `ifunc`.
//! C'est le travail de `ld.so`, qui est un programme entier. Chaque case
//! qui aurait dû recevoir une adresse de bibliothèque reçoit à la place
//! l'adresse d'une *borne* : l'appeler s'arrête net, en nommant la
//! fonction qu'on voulait appeler.

use std::collections::BTreeMap;
use std::ops::Range;

use namsdec_asm::elf::{self, Elf, RelocKind, align_up, page_down, page_up};
use namsdec_core::{Memory, Perms};

/// La taille d'une page. Celle d'x86-64, et celle dans laquelle un noyau
/// raisonne : un segment qui commence au milieu d'une page fait projeter
/// la page entière, et c'est pour cela qu'un programme peut lire le
/// rembourrage qui suit ses données sans que rien ne s'effondre.
pub const PAGE: u64 = 0x1000;

/// Ce que le chargeur réserve au tas d'un binaire ouvert.
///
/// Quatre mégaoctets : de quoi voir `malloc` travailler longtemps sans
/// faire de la mémoire simulée un poids pour la machine hôte.
const HEAP_ROOM: u64 = 4 * 1024 * 1024;

/// Ce que le chargeur réserve à `mmap`.
///
/// Seize mégaoctets : une bibliothèque C y prend ses tampons
/// d'entrée-sortie et ses grosses allocations, et un programme qui
/// travaille sur un fichier entier peut y tenir.
const MMAP_ROOM: u64 = 16 * 1024 * 1024;

/// La pile d'un binaire ouvert.
///
/// Un mégaoctet, là où un exercice se contente de huit kilo-octets : la
/// pile initiale d'un vrai processus porte déjà l'environnement et le
/// vecteur auxiliaire, et le code de démarrage d'une bibliothèque C y
/// creuse des cadres autrement plus profonds qu'un `push`/`pop` écrit à
/// la main.
const STACK_ROOM: u64 = 1024 * 1024;

/// La place laissée au bloc de contrôle du fil d'exécution, au-dessus du
/// pointeur de fil.
///
/// La `tcbhead_t` de la glibc fait quelques centaines d'octets ; une page
/// couvre largement, et ce qui dépasse ne coûte que des zéros.
const TCB_ROOM: u64 = 0x1000;

/// Le canari de pile que le chargeur installe en `fs:0x28`.
///
/// Une valeur fixe, et non du hasard : le même programme doit donner deux
/// fois le même résultat, sans quoi le retour en arrière ne veut plus
/// rien dire. Elle est reconnaissable à l'œil dans un vidage mémoire,
/// ce qui est exactement ce qu'on veut d'un canari qu'on étudie.
pub const STACK_GUARD: u64 = 0x0000_1d1e_a5e5_0000;

/// Les seize octets que le noyau laisse au programme par `AT_RANDOM`, et
/// dont la glibc tire son canari quand elle en pose un elle-même. Fixes,
/// pour la même raison.
const AT_RANDOM_BYTES: [u8; 16] = [
    0x9e, 0x37, 0x79, 0xb9, 0x7f, 0x4a, 0x7c, 0x15, 0xf3, 0x9c, 0xc0, 0x60, 0x5c, 0xed, 0xc8, 0x35,
];

/// Les entrées du vecteur auxiliaire que namsdec renseigne.
mod auxv {
    pub const NULL: u64 = 0;
    pub const PHDR: u64 = 3;
    pub const PHENT: u64 = 4;
    pub const PHNUM: u64 = 5;
    pub const PAGESZ: u64 = 6;
    pub const BASE: u64 = 7;
    pub const FLAGS: u64 = 8;
    pub const ENTRY: u64 = 9;
    pub const UID: u64 = 11;
    pub const EUID: u64 = 12;
    pub const GID: u64 = 13;
    pub const EGID: u64 = 14;
    pub const PLATFORM: u64 = 15;
    pub const HWCAP: u64 = 16;
    pub const CLKTCK: u64 = 17;
    pub const SECURE: u64 = 23;
    pub const RANDOM: u64 = 25;
    pub const EXECFN: u64 = 31;
}

/// Le plan mémoire d'un programme chargé — ce que `vmmap` montrerait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// La fenêtre simulée : là où commence le bloc d'octets.
    pub base: u64,
    pub size: usize,
    /// L'image du programme, pages comprises.
    pub image: Range<u64>,
    /// Le bloc TLS, et le pointeur de fil qui le suit.
    pub tls: Range<u64>,
    pub thread_pointer: u64,
    /// La page des fonctions qu'aucune bibliothèque ne fournit ici.
    pub externals: Range<u64>,
    /// Le tas : de son début à son plafond.
    pub heap: Range<u64>,
    /// L'arène de `mmap` : la mémoire hors du tas.
    pub mmap: Range<u64>,
    /// La pile, du plus bas qu'elle puisse descendre à son sommet.
    pub stack: Range<u64>,
}

impl Layout {
    /// Le plan d'un binaire ouvert depuis le disque.
    ///
    /// Tout se déduit de l'image : le programme d'abord, puis ce dont il
    /// aura besoin, dans l'ordre où un processus les rencontre — le tas
    /// qui monte, la pile qui descend, et une page de garde entre les
    /// deux pour que l'un ne dévore pas l'autre en silence.
    pub fn for_binary(load_addr: u64, image_len: usize, tls_memsz: usize) -> Self {
        let début = page_down(load_addr);
        let image = début..page_up(load_addr + image_len as u64);
        let tls_taille = page_up(tls_memsz as u64 + TCB_ROOM);
        let tls = image.end..image.end + tls_taille;
        // Le pointeur de fil est *au-dessus* du bloc TLS : sur x86-64,
        // les variables locales au fil se lisent en dessous de FS, à des
        // décalages négatifs. C'est le modèle « variante II », et s'en
        // écarter ferait lire au programme la mémoire d'à côté.
        let thread_pointer = tls.end - TCB_ROOM;
        let externals = tls.end..tls.end + PAGE;
        let heap = externals.end..externals.end + HEAP_ROOM;
        let mmap = heap.end..heap.end + MMAP_ROOM;
        // Une page de garde : le jour où la pile déborde, elle tombe sur
        // du vide et le dit, au lieu d'écraser le tas sans bruit.
        let stack = mmap.end + PAGE..mmap.end + PAGE + STACK_ROOM;
        Self {
            base: début,
            size: (stack.end - début) as usize,
            image,
            tls,
            thread_pointer,
            externals,
            heap,
            mmap,
            stack,
        }
    }

    /// Le sommet de la pile : quelques octets sous le haut de la zone,
    /// pour qu'une adresse écrite tout en haut reste lisible.
    pub fn stack_top(&self) -> u64 {
        self.stack.end - 0x100
    }
}

/// Ce que le chargeur a fait, dit dans l'ordre où il l'a fait.
///
/// Un chargeur muet est un chargeur qu'on ne peut pas mettre en cause. Ce
/// rapport est ce que namsdec montre après avoir ouvert un binaire, et
/// c'est lui qui explique la suite — y compris les arrêts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Combien de segments ont été projetés.
    pub segments: usize,
    /// Combien de relocations ont été appliquées.
    pub relocations: usize,
    /// Ce qu'il aurait fallu aller chercher dans une bibliothèque.
    pub external: Vec<String>,
    /// Combien de fonctions sont choisies par un résolveur `ifunc`.
    pub ifunc: usize,
    /// La taille du bloc TLS posé, zéro s'il n'y en avait pas.
    pub tls: usize,
    /// Les arguments passés au programme.
    pub argv: Vec<String>,
}

/// Un programme chargé, prêt à recevoir sa première instruction.
pub struct Loaded {
    pub memory: Memory,
    pub layout: Layout,
    pub entry: u64,
    pub rsp: u64,
    /// La base FS : le pointeur de fil, là où le programme ira chercher
    /// son canari.
    pub fs_base: u64,
    /// Les bornes posées à la place des fonctions de bibliothèque :
    /// adresse -> nom. Arriver sur l'une d'elles, c'est appeler cette
    /// fonction-là — namsdec en fournit une poignée, et nomme les autres.
    pub externals: BTreeMap<u64, String>,
    /// La borne sur laquelle `main` revient : son résultat y devient le
    /// code de sortie du programme, comme le ferait `__libc_start_main`.
    pub main_return: u64,
    pub report: Report,
}

/// Pourquoi un binaire n'a pas pu être chargé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Pas un ELF64, ou un ELF sans rien de chargeable — un `.o`, que
    /// `ld` n'a pas encore vu.
    NotLoadable,
    /// Il faudrait plus de mémoire que namsdec n'en simule.
    TooLarge { needed: usize },
}

/// Charge un ELF comme le noyau le ferait, et rend de quoi démarrer.
///
/// `name` est le nom sous lequel le programme se verra lui-même :
/// `argv[0]`, et `AT_EXECFN`. Un programme qui se plaint le cite, et le
/// voir juste vaut mieux que de le voir vide.
pub fn load(
    elf: &Elf,
    name: &str,
    args: &[String],
    env: &[String],
    max: usize,
) -> Result<Loaded, LoadError> {
    // Le plan AVANT l'image : `load_image` allouait, et le plafond
    // n'était contrôlé qu'ensuite, sur le résultat. Un en-tête qui
    // déclare un demi-gigaoctet obtenait donc son demi-gigaoctet pour
    // se voir refuser juste après.
    let Some(plan) = elf.load_plan() else {
        return Err(LoadError::NotLoadable);
    };
    let tls_memsz = elf.tls.map(|tls| tls.memsz).unwrap_or(0);
    let layout = Layout::for_binary(plan.addr, plan.len, tls_memsz);
    if layout.size > max {
        return Err(LoadError::TooLarge {
            needed: layout.size,
        });
    }
    // La disposition tient dans la fenêtre : l'image peut être bâtie.
    // Le plafond qu'on lui passe est celui qui vient d'être vérifié, et
    // la machine peut encore refuser — auquel cas on le dit.
    let (load_addr, mut image) = match elf.load_image(plan.len) {
        Ok(fait) => fait,
        Err(elf::ImageError::OutOfMemory { needed }) => {
            return Err(LoadError::TooLarge { needed });
        }
        Err(_) => return Err(LoadError::NotLoadable),
    };

    // Les relocations, avant même de poser l'image : c'est l'ordre du
    // vrai chargeur, et cela évite de devoir écrire deux fois dans la
    // mémoire simulée.
    let mut externals = BTreeMap::new();
    // La première borne de la page est réservée au retour de `main` :
    // c'est là que `__libc_start_main` fait revenir le programme.
    let main_return = layout.externals.start;
    let mut prochaine = layout.externals.start + 16;
    let mut bornes: BTreeMap<String, u64> = BTreeMap::new();
    let rapport = elf.relocate(load_addr, &mut image, 0, |reloc| {
        // Chaque nom reçoit *sa* borne, et toujours la même : deux cases
        // qui attendent `printf` doivent mener au même endroit, sans quoi
        // comparer deux pointeurs de fonction donnerait faux.
        let nom = if reloc.kind == RelocKind::Irelative {
            format!("résolveur ifunc {:#x}", reloc.addend)
        } else {
            reloc.symbol.clone()
        };
        if let Some(déjà) = bornes.get(&nom) {
            return Some(*déjà);
        }
        if prochaine + 16 > layout.externals.end {
            return None;
        }
        let adresse = prochaine;
        prochaine += 16;
        bornes.insert(nom.clone(), adresse);
        externals.insert(adresse, nom);
        Some(adresse)
    });

    let mut memory = Memory::new(layout.base, layout.size);
    memory
        .poke(load_addr, &image)
        .map_err(|_| LoadError::TooLarge {
            needed: layout.size,
        })?;

    map_regions(&mut memory, elf, &layout);
    let tls_posé = install_tls(&mut memory, elf, &layout);
    let mut report = Report {
        segments: elf.segments.len(),
        relocations: rapport.applied,
        external: rapport.external,
        ifunc: rapport.ifunc,
        tls: tls_posé,
        argv: vec![name.to_owned()],
    };
    let rsp = build_stack(&mut memory, elf, &layout, name, args, env, &mut report);

    Ok(Loaded {
        memory,
        entry: elf.entry,
        rsp,
        fs_base: layout.thread_pointer,
        layout,
        externals,
        main_return,
        report,
    })
}

/// Projette les zones : les segments d'abord, les sections par-dessus.
///
/// Les deux listes ne servent pas à la même chose. Les **segments**
/// décident : ce sont eux qui disent ce qui est projeté et avec quels
/// droits, en pages entières, y compris le rembourrage entre deux
/// sections. Les **sections** nomment : elles ne changent rien à ce qui
/// existe, mais elles remplacent « des données » par `.got`, et c'est
/// souvent toute la réponse à la question qu'on se pose.
fn map_regions(memory: &mut Memory, elf: &Elf, layout: &Layout) {
    for segment in &elf.segments {
        let début = page_down(segment.vaddr);
        let fin = page_up(segment.vaddr + segment.memsz as u64);
        memory.map(
            début..fin,
            Perms {
                read: segment.read,
                write: segment.write,
                execute: segment.execute,
            },
            "programme",
        );
    }

    for section in &elf.sections {
        if section.size == 0 {
            continue;
        }
        let plage = section.addr..section.addr + section.size as u64;
        // Seulement ce qui est réellement chargé. Une section TLS
        // (`.tbss`) porte une adresse qui ne désigne rien dans le
        // processus : son contenu est recopié par fil d'exécution, et la
        // projeter poserait une île de mémoire là où il n'y a rien.
        if !elf.segments.iter().any(|segment| {
            section.addr >= segment.vaddr
                && plage.end <= segment.vaddr + segment.memsz as u64
                && segment.memsz > 0
        }) {
            continue;
        }
        memory.map(
            plage,
            Perms {
                read: true,
                write: section.writable,
                execute: section.executable,
            },
            crate::region_label_named(&section.name, section.executable, section.writable),
        );
    }

    // Les bornes : lisibles et **inscriptibles**, mais pas exécutables.
    //
    // Inscriptibles parce qu'une bibliothèque ne fournit pas que des
    // fonctions : `stdout`, `errno`, `environ` sont des *données*, et un
    // programme écrit dedans. Leur borne doit donc être une vraie case
    // mémoire, sans quoi la première écriture serait refusée.
    //
    // Pas exécutables parce que rien ne s'y exécute : arriver sur la
    // borne d'une fonction est intercepté avant tout décodage, et ce qui
    // atterrit ailleurs dans la page est une erreur, qui doit le rester.
    memory.map(layout.externals.clone(), Perms::DATA, "bibliothèque");
    memory.map(layout.tls.clone(), Perms::DATA, "TLS");
    memory.map(layout.stack.clone(), Perms::DATA, "pile");
}

/// Pose le bloc TLS et le bloc de contrôle du fil.
///
/// Sur x86-64, le pointeur de fil (la base FS) montre le **haut** du
/// bloc : les variables locales au fil se lisent en dessous, à des
/// décalages négatifs, et le bloc de contrôle juste au-dessus. En
/// `fs:0x00` la glibc attend un pointeur vers elle-même, en `fs:0x28` le
/// canari de pile — la toute première fonction protégée le lit, et sans
/// lui un binaire moderne s'arrête avant d'avoir rien fait.
///
/// Rend la taille du modèle TLS posé.
fn install_tls(memory: &mut Memory, elf: &Elf, layout: &Layout) -> usize {
    let tp = layout.thread_pointer;
    if let Some(tls) = elf.tls {
        let taille = align_up(tls.memsz as u64, tls.align);
        let début = tp - taille;
        if let Some(offset) = file_slice(elf, tls.vaddr, tls.filesz) {
            let _ = memory.poke(début, offset);
        }
    }
    // Le pointeur vers soi-même : `fs:0` doit contenir l'adresse de
    // `fs:0`. Tout le code de la glibc qui manipule le TLS commence par
    // le relire.
    let _ = memory.poke(tp, &tp.to_le_bytes());
    let _ = memory.poke(tp + 0x28, &STACK_GUARD.to_le_bytes());
    elf.tls.map(|tls| tls.memsz).unwrap_or(0)
}

/// La tranche du fichier que désigne une adresse virtuelle.
fn file_slice(elf: &Elf, vaddr: u64, len: usize) -> Option<&[u8]> {
    let segment = elf.segments.iter().find(|segment| {
        vaddr >= segment.vaddr && vaddr + len as u64 <= segment.vaddr + segment.filesz as u64
    })?;
    let offset = segment.offset + (vaddr - segment.vaddr) as usize;
    elf.file.get(offset..offset + len)
}

/// Écrit la pile initiale, telle que le noyau la laisse à `_start`.
///
/// De haut en bas : les chaînes (le nom du programme, l'environnement, le
/// nom de la plateforme, les seize octets de hasard), puis le vecteur —
/// `argc`, les pointeurs `argv`, un zéro, les pointeurs d'environnement,
/// un zéro, enfin le vecteur auxiliaire terminé par `AT_NULL`.
///
/// Rend RSP : il pointe sur `argc`, aligné sur seize octets, comme
/// l'exige la convention d'appel — un `movaps` sur la pile, dans le code
/// de démarrage d'une bibliothèque C, ne pardonne pas le contraire.
fn build_stack(
    memory: &mut Memory,
    elf: &Elf,
    layout: &Layout,
    name: &str,
    args: &[String],
    env_extra: &[String],
    report: &mut Report,
) -> u64 {
    // Ce qu'un programme trouve quand on ne lui a rien dit. Cinq
    // variables : assez pour qu'une bibliothèque C se croie chez elle,
    // pas assez pour qu'on s'y perde.
    const ENVIRONNEMENT: [&str; 5] = [
        "LANG=C",
        "PATH=/usr/bin:/bin",
        "PWD=/",
        "HOME=/",
        "SHELL=/bin/sh",
    ];

    let mut curseur = layout.stack_top();
    let execfn = pose_texte(memory, &mut curseur, name);
    let argv0 = execfn;
    let plateforme = pose_texte(memory, &mut curseur, "x86_64");
    // Les variables : celles d'origine, puis celles qu'on a ajoutées.
    // Une variable donnée deux fois l'emporte sur celle d'origine —
    // `getenv` rend la première trouvée, et l'on veut que ce soit la
    // sienne.
    let mut variables: Vec<String> = env_extra
        .iter()
        .filter(|texte| !texte.trim().is_empty())
        .map(|texte| texte.trim().to_owned())
        .collect();
    for origine in ENVIRONNEMENT {
        let nom = origine.split('=').next().unwrap_or(origine);
        let déjà = variables
            .iter()
            .any(|v| v.split('=').next().unwrap_or(v) == nom);
        if !déjà {
            variables.push(origine.to_owned());
        }
    }
    let env: Vec<u64> = variables
        .iter()
        .map(|variable| pose_texte(memory, &mut curseur, variable))
        .collect();
    // Les arguments, après `argv[0]` : beaucoup de programmes — et la
    // plupart des crackmes — attendent leur mot de passe là plutôt que
    // sur l'entrée standard. Sans eux, on ne pouvait qu'assister à un
    // « usage: ... » et s'arrêter.
    let pointeurs: Vec<u64> = args
        .iter()
        .map(|argument| pose_texte(memory, &mut curseur, argument))
        .collect();
    let random = pose_octets(memory, &mut curseur, &AT_RANDOM_BYTES);
    report.argv = std::iter::once(name.to_owned())
        .chain(args.iter().cloned())
        .collect();

    let (phdr, phent, phnum) = elf.phdr.unwrap_or((0, 0, 0));
    let mut aux: Vec<(u64, u64)> = vec![
        (auxv::PHDR, phdr),
        (auxv::PHENT, phent as u64),
        (auxv::PHNUM, phnum as u64),
        (auxv::PAGESZ, PAGE),
        // Aucun interpréteur n'est chargé : la base de `ld.so` est nulle,
        // et le dire est plus honnête que d'inventer une adresse.
        (auxv::BASE, 0),
        (auxv::FLAGS, 0),
        (auxv::ENTRY, elf.entry),
        (auxv::UID, 1000),
        (auxv::EUID, 1000),
        (auxv::GID, 1000),
        (auxv::EGID, 1000),
        (auxv::PLATFORM, plateforme),
        // Les capacités du processeur : namsdec ne promet rien, ce qui
        // pousse une bibliothèque à choisir ses versions les plus
        // simples — précisément celles qu'on sait simuler.
        (auxv::HWCAP, 0),
        (auxv::CLKTCK, 100),
        (auxv::SECURE, 0),
        (auxv::RANDOM, random),
        (auxv::EXECFN, execfn),
    ];
    if phdr == 0 {
        // Sans en-têtes projetés, autant ne pas mentir : une entrée
        // `AT_PHDR` nulle vaut mieux qu'un pointeur vers rien.
        aux.retain(|(clé, _)| !matches!(*clé, auxv::PHDR | auxv::PHENT | auxv::PHNUM));
    }
    aux.push((auxv::NULL, 0));

    // Le vecteur, compté en mots de huit octets : argc, argv + zéro,
    // env + zéro, puis les paires du vecteur auxiliaire.
    let mots = 1 + 1 + pointeurs.len() + 1 + env.len() + 1 + aux.len() * 2;
    let mut rsp = curseur - mots as u64 * 8;
    // La convention d'appel veut RSP aligné sur seize octets à l'entrée
    // du programme.
    rsp &= !15;

    let mut at = rsp;
    let mut mot = |memory: &mut Memory, valeur: u64| {
        let _ = memory.poke(at, &valeur.to_le_bytes());
        at += 8;
    };
    mot(memory, 1 + pointeurs.len() as u64); // argc
    mot(memory, argv0);
    for pointeur in &pointeurs {
        mot(memory, *pointeur);
    }
    mot(memory, 0);
    for variable in &env {
        mot(memory, *variable);
    }
    mot(memory, 0);
    for (clé, valeur) in &aux {
        mot(memory, *clé);
        mot(memory, *valeur);
    }
    rsp
}

/// Dépose des octets sous le curseur et rend leur adresse.
fn pose_octets(memory: &mut Memory, curseur: &mut u64, octets: &[u8]) -> u64 {
    *curseur -= octets.len() as u64;
    // Aligné sur huit : rien ne l'exige pour des chaînes, mais un vidage
    // mémoire de la pile initiale se lit infiniment mieux.
    *curseur &= !7;
    let _ = memory.poke(*curseur, octets);
    *curseur
}

/// Dépose une chaîne terminée par un zéro, comme le noyau les pose.
fn pose_texte(memory: &mut Memory, curseur: &mut u64, texte: &str) -> u64 {
    let mut octets = texte.as_bytes().to_vec();
    octets.push(0);
    pose_octets(memory, curseur, &octets)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le plan mémoire se lit comme celui d'un vrai processus : le
    /// programme en bas, la pile en haut, et ce qui monte entre les deux
    /// sans jamais se recouvrir.
    #[test]
    fn the_layout_reads_like_a_real_process() {
        let layout = Layout::for_binary(0x1000, 0x4321, 24);

        assert_eq!(layout.base, 0x1000, "la fenêtre commence à l'image");
        assert_eq!(layout.image.start % PAGE, 0, "et sur une page");
        assert_eq!(layout.image.end % PAGE, 0);
        assert!(
            layout.image.end >= 0x1000 + 0x4321,
            "l'image couvre tout ce qui est chargé"
        );

        // Chaque zone commence où la précédente finit, dans l'ordre où un
        // processus les rencontre.
        assert_eq!(layout.tls.start, layout.image.end);
        assert_eq!(layout.externals.start, layout.tls.end);
        assert_eq!(layout.heap.start, layout.externals.end);
        assert_eq!(layout.mmap.start, layout.heap.end);
        // Sauf la pile : une page de garde l'en sépare, pour qu'un
        // débordement tombe sur du vide et le dise.
        assert_eq!(
            layout.stack.start,
            layout.mmap.end + PAGE,
            "une page de garde sous la pile"
        );

        // Le pointeur de fil est dans le bloc TLS, et le sommet de pile
        // dans la pile.
        assert!(layout.tls.contains(&layout.thread_pointer));
        assert!(layout.stack.contains(&layout.stack_top()));
        assert_eq!(
            layout.size,
            (layout.stack.end - layout.base) as usize,
            "la fenêtre couvre tout, et rien de plus"
        );
    }

    /// Un programme plus gros ne fait que décaler ce qui le suit : le
    /// plan ne dépend d'aucune adresse écrite en dur.
    #[test]
    fn a_bigger_program_only_pushes_the_rest_up() {
        let petit = Layout::for_binary(0, 0x1000, 0);
        let gros = Layout::for_binary(0, 0x100000, 0);
        assert!(gros.heap.start > petit.heap.start);
        assert_eq!(
            gros.heap.end - gros.heap.start,
            petit.heap.end - petit.heap.start,
            "le tas garde la même taille"
        );
        assert_eq!(
            gros.stack.end - gros.stack.start,
            petit.stack.end - petit.stack.start,
            "la pile aussi"
        );
    }
}
