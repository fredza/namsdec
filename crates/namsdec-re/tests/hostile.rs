//! L'invariant de robustesse de tout l'outil : **aucune entrée ne fait
//! paniquer un analyseur**.
//!
//! C'est la première exigence d'un outil d'analyse de code malveillant.
//! Les fichiers qu'on y jette sont délibérément malformés — c'est
//! souvent le but d'un empaqueteur que de casser les lecteurs — et un
//! analyseur qui tombe dessus n'analyse plus rien. Pire : en *debug* un
//! débordement d'entier panique, et en *release* il rend un index faux
//! et une analyse silencieusement fausse. Les deux sont inacceptables,
//! et le premier est le seul qu'un test puisse voir.
//!
//! Un tel contrôle existait déjà, dans `imports.rs`, après qu'un fuzzing
//! léger y eut trouvé deux paniques. Il ne couvrait que trois
//! analyseurs. Celui-ci les couvre **tous**, le désassembleur compris —
//! qui reçoit des octets arbitraires par construction.
//!
//! ## Le germe est fabriqué ici, et c'est voulu
//!
//! Le contrôle d'origine partait de `/bin/true`, et s'abstenait quand il
//! ne le trouvait pas : sur une machine sans lui, il passait sans rien
//! prouver. Celui-ci **fabrique** son ELF, octet par octet : en-tête,
//! segments, sections, table de symboles, relocations et entrées de
//! liaison. Il tourne donc partout de la même façon, et il mène le
//! fuzzing jusque dans les tables — là où les débordements se cachent.
//! Un binaire du système s'y ajoute quand il y en a un, pour la variété,
//! mais le contrôle ne dépend pas de lui.
//!
//! ## Ce qu'il prouve, et ce qu'il ne prouve pas
//!
//! Il prouve qu'aucun analyseur ne panique. Il ne dit **rien** de la
//! justesse de ce qu'ils rendent sur un fichier cassé : une liste
//! d'imports tirée d'un en-tête corrompu n'a aucune raison d'être juste,
//! et l'exiger n'aurait pas de sens. Ne pas tomber est le contrat ; être
//! juste est le contrat sur un fichier valide, et c'est ailleurs qu'on
//! l'éprouve.

mod commun;

use commun::{BASE, germe};

/// Le germe est un ELF que namsdec lit vraiment.
///
/// Sans ce contrôle, le fuzzing pourrait tourner des milliers de fois
/// sur un fichier que tous les analyseurs rejettent à la porte, et ne
/// rien prouver du tout — c'est le défaut qu'avait le contrôle d'origine
/// sur une machine sans `/bin/true`.
#[test]
fn the_seed_is_an_elf_namsdec_really_reads() {
    let f = germe();
    let scan = namsdec_re::elf_scan::scan(&f).expect("le germe est un ELF64 lisible");
    assert_eq!(scan.entry, BASE + 0x100);
    // Les huit sections nommées : la section nulle d'index zéro ne
    // décrit rien, et l'analyse ne la garde pas.
    let noms_sections: Vec<&str> = scan.sections.iter().map(|s| s.name.as_str()).collect();
    for voulue in [
        ".text",
        ".plt",
        ".rela.plt",
        ".dynsym",
        ".dynstr",
        ".rodata",
    ] {
        assert!(
            noms_sections.contains(&voulue),
            "« {voulue} » est vue : {noms_sections:?}"
        );
    }
    assert!(
        !scan.writable_executable().is_empty(),
        "le segment inscriptible et exécutable est vu"
    );

    // Les imports : c'est le chemin le plus profond, celui où la panique
    // d'origine se cachait. S'il ne mène à rien, le fuzzing ne descendra
    // pas jusque-là.
    let imports = namsdec_re::imports::scan(&f);
    let noms: Vec<&str> = imports.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(noms, vec!["socket", "ptrace"], "{imports:?}");

    // Et de quoi faire travailler les analyseurs d'en haut.
    assert!(
        namsdec_re::strings::scan_elf(&f)
            .unwrap_or_default()
            .iter()
            .any(|chaîne| chaîne.text.contains("exemple.ru")),
        "les données portent un indicateur"
    );
    let signaux = namsdec_re::signals::gather(&f, &[], "fr");
    assert!(
        signaux
            .iter()
            .any(|s| s.fr == "Segment inscriptible et exécutable"),
        "{signaux:?}"
    );
}

/// Un générateur reproductible : un échec se rejoue avec la même graine,
/// sans dépendre d'un fichier temporaire.
struct Hasard(u64);

impl Hasard {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Un fichier abîmé à partir de `base`.
///
/// Trois façons de l'abîmer, parce qu'elles n'atteignent pas les mêmes
/// chemins : des octets changés au hasard (les champs deviennent
/// quelconques), une troncature (les tables pointent au-delà de la fin),
/// et des valeurs extrêmes posées sur huit octets alignés (les
/// multiplications débordent — c'est ainsi que la panique d'origine a
/// été trouvée).
fn abîme(base: &[u8], rnd: &mut Hasard) -> Vec<u8> {
    let mut v = base.to_vec();
    if v.is_empty() {
        return v;
    }
    for _ in 0..(rnd.next() % 48) {
        let i = rnd.next() as usize % v.len();
        v[i] = rnd.next() as u8;
    }
    // Des valeurs qui débordent, posées là où un champ de huit octets
    // peut se trouver : `u64::MAX`, de quoi faire déborder toute
    // multiplication qui ne se vérifie pas.
    for _ in 0..(rnd.next() % 4) {
        let i = (rnd.next() as usize % v.len()) & !7;
        if i + 8 <= v.len() {
            let valeur = match rnd.next() % 3 {
                0 => u64::MAX,
                1 => u64::MAX / 2,
                _ => 0xffff_ffff,
            };
            v[i..i + 8].copy_from_slice(&valeur.to_le_bytes());
        }
    }
    if rnd.next().is_multiple_of(3) {
        let coupe = rnd.next() as usize % v.len();
        v.truncate(coupe);
    }
    // Le nombre magique est conservé : c'est ce qui mène l'analyse
    // au-delà de la porte, là où les débordements se cachent. Un bruit
    // pur ne franchirait pas les quatre premiers octets.
    if v.len() >= 4 {
        v[..4].copy_from_slice(b"\x7fELF");
    }
    v
}

/// **Tous** les analyseurs, sur ces octets-là.
///
/// Une seule fonction, et c'est le point : un analyseur ajouté demain
/// s'ajoute ici, et se trouve éprouvé par tous les germes d'un coup.
/// Les valeurs rendues sont ignorées — sur un fichier cassé, elles n'ont
/// aucune raison d'être justes. Ce qu'on éprouve, c'est que l'appel
/// revienne.
fn tout_analyser(octets: &[u8]) {
    // --- Ce qui lit le fichier -------------------------------------
    let scan = namsdec_re::elf_scan::scan(octets);
    let _ = namsdec_re::imports::scan(octets);
    // Des adresses prises au hasard, dedans et dehors : la conversion
    // adresse → position dans le fichier fait de l'arithmétique sur des
    // champs du fichier.
    for vaddr in [0, 1, BASE, BASE + 0x100, u64::MAX, u64::MAX - 0x1000] {
        let _ = namsdec_re::imports::file_offset(octets, vaddr);
    }
    let _ = namsdec_re::slack::totals(&namsdec_re::slack::find(octets));
    let chaînes = namsdec_re::strings::scan_elf(octets).unwrap_or_default();
    let _ = namsdec_re::strings::scan(octets, BASE);
    let _ = namsdec_re::ioc::from_strings(&chaînes);
    let _ = namsdec_re::entropy::shannon(octets);
    let _ = namsdec_re::entropy::profile(octets, 64);
    let _ = namsdec_re::packer::detect(octets);
    let _ = namsdec_re::digest::crc32(octets);
    let _ = namsdec_re::digest::sha256(octets);
    let _ = namsdec_re::gadgets::find(octets, BASE, 64);

    // --- Le désassembleur, qui reçoit des octets arbitraires -------
    //
    // Borné à quatre kibioctets : ce qu'on éprouve est le décodage d'une
    // suite quelconque, non la patience du test.
    let tranche = &octets[..octets.len().min(4096)];
    let disasm = namsdec_disasm::disassemble(tranche, BASE);
    for instruction in &disasm {
        let _ = namsdec_disasm::decode_one(&instruction.bytes, instruction.address);
        if let Some(décodée) = namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
        {
            let _ = namsdec_disasm::format_one(&décodée);
            let _ = namsdec_disasm::fields_of(&décodée, &instruction.bytes);
        }
    }

    // --- Ce qui lit le désassemblage -------------------------------
    let entrée = scan.as_ref().map(|s| s.entry).unwrap_or(BASE);
    let fonctions = namsdec_re::functions::discover(&disasm, entrée);
    let _ = namsdec_re::functions::by_importance(&fonctions);
    let _ = namsdec_re::functions::main_of(&disasm, entrée);
    let xrefs = namsdec_re::xrefs::scan(&disasm, BASE..BASE + tranche.len() as u64);
    let _ = xrefs.functions();
    let _ = xrefs.labels();
    let _ = xrefs.loops();
    let _ = namsdec_re::patterns::scan(&disasm);
    let appels = namsdec_re::syscalls::scan(&disasm);
    for appel in &appels {
        let _ = appel.family();
    }
    let _ = namsdec_re::syscalls::reads_input(&disasm);
    let _ = namsdec_re::gadgets::from_disasm(&disasm, 64);
    let cfg = namsdec_re::cfg::build(&disasm);
    let _ = cfg.to_dot();
    let _ = cfg.layers();
    let lignes = namsdec_re::structure::pseudo_code(&cfg, &disasm, "fr");
    let _ = namsdec_re::structure::en_texte(&lignes);
    if let Some(scan) = scan.as_ref() {
        let _ = namsdec_re::literals::scan(&disasm, &scan.sections, octets);
    }
    // Remonter l'origine d'un registre depuis la première et la dernière
    // instruction : le parcours remonte des chaînes de dépendances que
    // des sauts quelconques rendent tordues.
    for instruction in [disasm.first(), disasm.last()].into_iter().flatten() {
        for registre in ["rax", "rdi", "rsp", "r15"] {
            let _ = namsdec_re::provenance::trace(&disasm, instruction.address, registre);
        }
    }

    // --- Les synthèses, qui rassemblent tout le reste --------------
    let _ = namsdec_re::signals::gather(octets, &disasm, "fr");
    let _ = namsdec_re::signals::gather(octets, &disasm, "en");
    if let Some(scan) = scan.as_ref() {
        let _ = namsdec_re::report::markdown(scan, octets, &disasm, "fr", "x");
        let _ = namsdec_re::report::markdown(scan, octets, &disasm, "en", "x");
    }
}

/// Aucun analyseur ne panique sur un ELF hostile.
///
/// Le contrat de robustesse de tout l'outil, et le seul test qui le
/// tient. Il ne regarde aucune valeur : le simple fait que chaque appel
/// revienne *est* la garantie.
#[test]
fn no_analyser_ever_panics_on_a_hostile_elf() {
    let mut rnd = Hasard(0x00C0_FFEE_1234);
    let germe = germe();

    // Le germe fabriqué, d'abord : il tourne partout, et il mène
    // l'analyse jusque dans les tables.
    // Deux cent cinquante tours : chaque tour fait travailler une
    // vingtaine d'analyseurs, le désassembleur et les deux rapports. Le
    // compte est choisi pour que le contrôle reste de l'ordre de
    // quelques secondes en debug — un test qu'on désactive parce qu'il
    // est long ne protège plus rien.
    for _ in 0..250 {
        tout_analyser(&abîme(&germe, &mut rnd));
    }

    // Puis quelques cas dégénérés, qu'un hasard mettrait longtemps à
    // produire et qui sont précisément ceux qu'on oublie en écrivant un
    // analyseur.
    for cas in [
        vec![],
        b"\x7fELF".to_vec(),
        b"\x7fELF\x02\x01\x01".to_vec(),
        vec![0u8; 64],
        vec![0xffu8; 64],
        vec![0xffu8; 4096],
        {
            // Un en-tête qui annonce soixante-cinq mille sections et
            // n'en porte aucune.
            let mut f = germe.clone();
            f[0x3c..0x3e].copy_from_slice(&u16::MAX.to_le_bytes());
            f
        },
        {
            // Une table de sections au-delà de la fin du fichier.
            let mut f = germe.clone();
            f[0x28..0x30].copy_from_slice(&u64::MAX.to_le_bytes());
            f
        },
        {
            // Une entrée de symbole de taille zéro : de quoi diviser ou
            // boucler sans fin chez qui ne s'en méfie pas.
            let mut f = germe.clone();
            let dynsym = 0x300 + 4 * 64;
            f[dynsym + 0x38..dynsym + 0x40].copy_from_slice(&0u64.to_le_bytes());
            f
        },
        {
            // Un index de symbole énorme dans une relocation : c'est
            // exactement la forme de la panique d'origine.
            let mut f = germe.clone();
            f[0x148..0x150].copy_from_slice(&(((u32::MAX as u64) << 32) | 7).to_le_bytes());
            f
        },
    ] {
        tout_analyser(&cas);
    }

    // Et un binaire du système quand il y en a un : de la variété, pour
    // des tables bien plus grandes que celles du germe. Son absence ne
    // change rien à ce que le contrôle prouve.
    for chemin in ["/bin/true", "/bin/ls"] {
        let Ok(vrai) = std::fs::read(chemin) else {
            continue;
        };
        for _ in 0..40 {
            tout_analyser(&abîme(&vrai, &mut rnd));
        }
    }
}
