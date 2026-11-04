//! Faire tourner le programme : assembler, charger, pas à pas, courir.
//!
//! Rien ici ne dessine. C'est la moitié de l'application qui SAIT — où
//! en est le processeur, ce qu'il a déjà exécuté, ce qu'il retiendra
//! pour le pas en arrière — et les fenêtres n'en sont que le regard.
//!
//! Le point d'exécution reste dans le crate `namsdec-emu` ; ce fichier
//! est la couturière entre lui et l'éditeur : il tient le tampon, les
//! points d'arrêt, la trace, et il traduit chaque erreur en une phrase
//! que quelqu'un qui apprend peut lire.

use super::*;

impl NamsdecApp {
    /// Assemble, et attend que ce soit fait.
    ///
    /// Le geste complet, pour qui a besoin du programme dans la foulée.
    /// L'interface ne l'emploie pas : elle lance par
    /// [`NamsdecApp::start_assemble`] et laisse
    /// [`NamsdecApp::tick_assemble`] récolter à l'image où le résultat
    /// arrive. C'est le même travail, dans le même fil, avec le même
    /// délai et la même récolte — seule l'attente diffère, et c'est
    /// elle qu'une fenêtre ne peut pas se permettre.
    ///
    /// **Réservé aux contrôles**, et c'est dit plutôt que caché : rien
    /// en production n'attend un assemblage de bout en bout depuis que
    /// P2 l'a mis en fond. Ce que les contrôles éprouvent par ici reste
    /// le chemin de production à l'attente près — `start_assemble` pose
    /// le travail, `tick_assemble` le récolte, et la révision tranche —
    /// ; seule la boucle d'attente leur appartient, parce qu'un
    /// contrôle a besoin d'un état arrêté pour regarder.
    ///
    /// Une action qui exige un programme prêt passe, elle, par
    /// [`NamsdecApp::assurer_le_programme`].
    #[cfg(test)]
    pub(crate) fn do_assemble(&mut self) {
        self.start_assemble();
        self.attendre_l_assemblage();
    }

    /// Lance l'assemblage dans un fil à part et rend la main aussitôt.
    ///
    /// Pourquoi en fond, alors qu'un assemblage ordinaire prend quatre
    /// millisecondes : parce qu'une seule ligne valide suffit à le
    /// faire durer. `times 2000000000 nop` occupe nasm plus de vingt
    /// secondes, mesuré, et l'interface n'avait alors ni image, ni
    /// clavier, ni Stop. Le délai du lanceur borne la casse à dix
    /// secondes ; ceci rend la fenêtre vivante pendant ces dix
    /// secondes, et donne de quoi en sortir avant.
    pub(crate) fn start_assemble(&mut self) {
        use std::sync::atomic::AtomicBool;
        use std::sync::{Arc, Mutex};

        // Réassembler un listing de binaire est presque toujours une
        // erreur de geste — un Ctrl+Maj+B par habitude. On le dit, et
        // l'on ne touche à rien : ce qui est chargé vaut mieux que ce
        // qu'on en referait.
        if self.buffer_came_from_a_binary() {
            self.status = Status::Error(strings(self.lang).binary_no_reassemble.to_owned());
            return;
        }
        // Un seul à la fois. Le précédent est arrêté plutôt que laissé
        // courir : son résultat serait de toute façon écarté par la
        // révision, et deux nasm pour un seul tampon ne servent
        // personne.
        self.cancel_assemble();

        let source = self.source.clone();
        let arrêt = Arc::new(AtomicBool::new(false));
        let résultat = Arc::new(Mutex::new(None));
        let (drapeau, boîte) = (Arc::clone(&arrêt), Arc::clone(&résultat));
        std::thread::spawn(move || {
            let bornes = namsdec_asm::Bornes::en(namsdec_asm::DÉLAI).arrêtable(drapeau);
            let rendu = namsdec_asm::assemble_within(&source, BASE_ADDR, &bornes);
            // Le verrou empoisonné ne fait rien perdre d'autre que ce
            // travail-ci : la récolte le verra vide et conclura que le
            // fil a échoué.
            if let Ok(mut case) = boîte.lock() {
                *case = Some(rendu);
            }
        });
        self.assemble_job = Some(AssembleJob {
            // LA RÉVISION AU DÉPART, et c'est tout l'objet du champ : le
            // texte peut changer, l'onglet peut changer — `restore_tab`
            // passe par `set_source`, qui fait avancer la révision —, et
            // un résultat qui arrive après ne décrit plus ce qui est à
            // l'écran. Le rattacher quand même, c'est donner à lire des
            // adresses et des numéros de ligne qui désignent un autre
            // programme.
            rev: self.texte_rev,
            arrêt,
            résultat,
            démarré: std::time::Instant::now(),
        });
        self.job = Some(BackgroundJob {
            label: strings(self.lang).job_assembling.to_owned(),
            started: std::time::Instant::now(),
            limit: Some(namsdec_asm::DÉLAI),
        });
        self.status = Status::Assembling;
    }

    /// Arrête l'assemblage en cours, s'il y en a un.
    ///
    /// Le drapeau est levé ; le fil le voit au prochain battement du
    /// lanceur — vingt millisecondes — et tue le groupe de processus.
    /// On n'attend pas ce fil : il écrira son résultat dans une boîte
    /// que plus personne ne lit.
    pub(crate) fn cancel_assemble(&mut self) {
        if let Some(job) = self.assemble_job.take() {
            job.arrêt.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if matches!(self.status, Status::Assembling) {
            self.status = Status::Idle;
        }
        self.job = None;
    }

    /// Y a-t-il un assemblage en cours ?
    pub(crate) fn assembling(&self) -> bool {
        self.assemble_job.is_some()
    }

    /// Récolte le résultat s'il est arrivé. À appeler une fois par
    /// image.
    pub(crate) fn tick_assemble(&mut self) {
        let Some(job) = &self.assemble_job else {
            return;
        };
        let Some(rendu) = job
            .résultat
            .lock()
            .ok()
            .and_then(|mut case| case.take())
            .or_else(|| {
                // Le verrou empoisonné : le fil a paniqué, et personne
                // ne posera jamais de résultat. Mieux vaut le dire que
                // laisser une barre tourner pour toujours.
                job.résultat
                    .is_poisoned()
                    .then_some(Err(namsdec_asm::AsmError::Cancelled))
            })
        else {
            return;
        };
        let (rev, démarré) = (job.rev, job.démarré);
        self.assemble_job = None;
        self.job = None;
        // PÉRIMÉ : le texte a changé pendant que nasm travaillait, ou
        // l'on a changé d'onglet. Le résultat décrit un programme que
        // personne ne regarde plus ; on le jette, et l'on ne touche ni
        // au listing, ni aux diagnostics, ni à l'émulateur de celui qui
        // est à l'écran.
        if rev != self.texte_rev {
            if matches!(self.status, Status::Assembling) {
                self.status = Status::Idle;
            }
            return;
        }
        self.apply_assembled(rendu, démarré.elapsed());
        // LE PANNEAU S'OUVRE ICI, et nulle part ailleurs : c'est le seul
        // endroit où les diagnostics viennent d'arriver. Posé après le
        // lancement, il montrait ceux de l'assemblage PRÉCÉDENT — une
        // erreur corrigée restait affichée, une erreur nouvelle
        // n'ouvrait rien.
        self.show_diagnostics_if_any();
    }

    /// Attend la fin de l'assemblage en cours, s'il y en a un.
    ///
    /// Bornée par le délai du lanceur, et d'une marge : le fil tue
    /// l'outil passé ce délai et pose son résultat, donc l'attente se
    /// termine d'elle-même. La marge couvre le battement de vingt
    /// millisecondes et le délai de grâce des lecteurs.
    pub(crate) fn attendre_l_assemblage(&mut self) {
        let fin =
            std::time::Instant::now() + namsdec_asm::DÉLAI + std::time::Duration::from_secs(2);
        while self.assemble_job.is_some() {
            self.tick_assemble();
            if self.assemble_job.is_none() {
                return;
            }
            if std::time::Instant::now() >= fin {
                // Le fil n'a pas rendu dans le temps qu'il s'était lui
                // même donné : on cesse d'attendre et on le dit, plutôt
                // que de boucler sans fin.
                self.cancel_assemble();
                self.status = Status::Error(errors::asm(
                    self.lang,
                    &namsdec_asm::AsmError::TooLong {
                        tool: "nasm",
                        seconds: namsdec_asm::DÉLAI.as_secs(),
                    },
                ));
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// Le panneau bas se met sur ce qui vient de se passer : les
    /// diagnostics si nasm a eu quelque chose à dire, la sortie du
    /// programme sinon. Sans ça, « Erreur d'assemblage » s'affichait dans
    /// la barre de statut sans que la raison soit nulle part en vue.
    pub(crate) fn show_diagnostics_if_any(&mut self) {
        if !self.diagnostics.is_empty() {
            self.show_console = true;
            self.console_tab = ConsoleTab::Diagnostics;
        } else if self.console_tab == ConsoleTab::Diagnostics {
            // Plus rien à signaler : on rend la place à la console, d'où
            // repartira l'exécution.
            self.console_tab = ConsoleTab::Output;
        }
    }

    /// Ce qu'on fait du résultat, une fois qu'il est là et qu'il
    /// concerne bien le texte à l'écran.
    pub(crate) fn apply_assembled(
        &mut self,
        rendu: Result<namsdec_asm::AssembleOutput, AsmError>,
        durée: std::time::Duration,
    ) {
        let _ = durée;
        match rendu {
            Ok(output) => {
                let disasm = disassemble_code(&output);
                let addr_to_line = output.line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
                self.diagnostics = output.diagnostics.clone();
                // Ce que nasm ne dit pas : un programme qui ne s'arrête
                // pas, une écriture dans une zone en lecture seule. Ces
                // deux fautes-là passent l'assemblage sans un mot et ne
                // se manifestent qu'à l'exécution — ou pire, pas du tout.
                self.diagnostics
                    .extend(namsdec_asm::lint::check(&output, self.lang == Lang::Fr));
                self.assemble_state = Some(AssembleState {
                    output,
                    disasm,
                    addr_to_line,
                    // Ce programme-ci vient d'un source : il est à nous.
                    from_binary: None,
                    octets_du_code: Default::default(),
                });
                self.note_program_io();
                // Et ce que namsdec ne saura pas faire : un appel
                // système qu'il ne simule pas arrête le programme au
                // milieu. Le dire à l'assemblage, avec sa ligne, évite
                // de chercher pourquoi l'exécution s'interrompt là.
                self.diagnostics.extend(self.unsimulated_syscalls());
                self.status = Status::AssembleOk;
                self.rebuild_emulator();
            }
            // Les diagnostics de l'assemblage précédent n'expliquent rien
            // de ces échecs-là. Laissés en place, `show_diagnostics_if_any`
            // ouvrait le panneau dessus et les présentait, compte à
            // l'appui, comme la cause de l'erreur affichée.
            Err(AsmError::NasmNotFound) => {
                self.diagnostics.clear();
                self.status = Status::NasmMissing;
            }
            Err(
                e @ (AsmError::LinkerNotFound
                | AsmError::LinkFailed { .. }
                | AsmError::UnreadableProgram),
            ) => {
                self.diagnostics.clear();
                self.assemble_state = None;
                self.emulator = None;
                self.status = Status::Error(errors::asm(self.lang, &e));
            }
            Err(AsmError::Failed { diagnostics }) => {
                self.diagnostics = diagnostics;
                self.assemble_state = None;
                self.emulator = None;
                self.status = Status::AssembleFailed;
            }
            Err(e @ AsmError::Io(_)) => {
                self.diagnostics.clear();
                self.status = Status::Error(errors::asm(self.lang, &e));
            }
            // L'outil a été arrêté : ce qui était assemblé avant ne
            // décrit plus ce texte-ci, et le garder ferait pointer la
            // gouttière sur des lignes qui ont bougé.
            Err(e @ AsmError::TooLong { .. }) => {
                self.diagnostics.clear();
                self.assemble_state = None;
                self.emulator = None;
                self.status = Status::Error(errors::asm(self.lang, &e));
            }
            // Arrêté à la demande : on ne dit pas « erreur », on ne dit
            // rien de plus que ce qui s'est passé.
            Err(AsmError::Cancelled) => {
                self.diagnostics.clear();
                self.status = Status::Note(|s| s.assemble_cancelled);
            }
        }
    }

    /// Retient l'état des registres avant de laisser le programme
    /// avancer.
    /// Ramène la ligne courante sous les yeux au prochain rendu.
    pub(crate) fn follow_execution(&mut self) {
        self.follow_rip = true;
    }

    /// Combien de fois chaque ligne source a été exécutée.
    ///
    /// L'émulateur compte par adresse ; une ligne en produit parfois
    /// plusieurs, et c'est la ligne qu'on lit. Vide tant que rien n'a
    /// tourné — auquel cas la gouttière ne réserve aucune place.
    /// Les appels système que le programme contient et que namsdec
    /// n'exécute pas.
    ///
    /// `mmap`, `open`, `fork` : namsdec les nomme mais ne les fait pas —
    /// il n'a ni fichiers ni processus à offrir. Un programme qui en
    /// contient un s'arrête dessus, et rien dans le code ne le laisse
    /// deviner. C'est exactement le genre de chose qu'un atelier doit
    /// dire avant, pas pendant.
    pub(crate) fn unsimulated_syscalls(&self) -> Vec<namsdec_asm::Diagnostic> {
        let Some(state) = &self.assemble_state else {
            return Vec::new();
        };
        namsdec_re::syscalls::scan(&state.disasm)
            .into_iter()
            .filter_map(|call| {
                let number = call.number?;
                if namsdec_re::table::is_simulated(number) {
                    return None;
                }
                let nom = namsdec_re::table::syscall_name(number)?;
                let line = state.addr_to_line.get(&call.addr).copied().unwrap_or(0);
                let kind = namsdec_asm::LintKind::UnsimulatedSyscall {
                    name: nom.to_owned(),
                    number,
                };
                Some(namsdec_asm::Diagnostic {
                    line,
                    severity: namsdec_asm::Severity::Warning,
                    message: kind.message(self.lang == Lang::Fr),
                    kind: Some(kind),
                })
            })
            .collect()
    }

    /// Relit ce que le programme chargé demande au monde extérieur.
    ///
    /// Une fois par programme, et non à chaque image : remonter le
    /// numéro d'un `syscall` coûte un décodage par instruction, ce qui
    /// se paie cher sur un binaire du système.
    pub(crate) fn note_program_io(&mut self) {
        let (mut lit, mut écrit) = (false, false);
        if let Some(state) = &self.assemble_state {
            for call in namsdec_re::syscalls::scan(&state.disasm) {
                match call.number {
                    // Un numéro illisible d'ici — calculé, lu en
                    // mémoire — peut être n'importe lequel : on ne
                    // refuse pas la saisie à un programme qui l'attend
                    // peut-être.
                    None => {
                        lit = true;
                        écrit = true;
                    }
                    Some(0) => lit = true,
                    Some(1) => écrit = true,
                    _ => {}
                }
            }
        }
        // Ce qu'un binaire lié dynamiquement réclame ne passe par aucun
        // `syscall` : il appelle `fgets` ou `scanf`, que namsdec fournit
        // lui-même. Sans regarder ces noms-là, rien ne disait qu'il
        // demanderait quelque chose, et la ligne de saisie n'apparaissait
        // qu'au moment où il se bloquait dessus.
        if let Some(binaire) = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
            && binaire
                .externals
                .values()
                .any(|nom| namsdec_emu::name_reads_input(nom))
        {
            lit = true;
        }
        self.program_reads = lit;
        self.program_writes = écrit;
    }

    /// Ce que le programme a fait, en phrases — seulement ce qui se
    /// compte.
    ///
    /// La moitié de l'explication d'un échec qui ne se mesurait pas :
    /// quelle branche il a prise, où la boucle s'est arrêtée. Les faits
    /// viennent de `namsdec_re::explain`, qui refuse de conclure là où
    /// plusieurs chemins se rejoignent ; ici on ne fait que les dire
    /// avec le numéro de ligne, qui est ce que la personne regarde.
    ///
    /// Deux au plus : une liste de quinze lignes sous un but manqué
    /// n'explique plus rien, elle noie.
    pub(crate) fn ce_que_le_programme_a_fait(&self) -> Vec<String> {
        let s = strings(self.lang);
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            return Vec::new();
        };
        let passages: BTreeMap<u64, u64> =
            emulator.visits().iter().map(|(a, c)| (*a, *c)).collect();
        let cfg = namsdec_re::cfg::build(&state.disasm);
        let ligne_de = |adresse: u64| -> Option<u32> { state.addr_to_line.get(&adresse).copied() };
        namsdec_re::explain::ce_qui_a_tourné(&cfg, &passages)
            .into_iter()
            .filter_map(|fait| match fait {
                namsdec_re::explain::Fait::Boucle { adresse, tours } => {
                    let ligne = ligne_de(adresse)?;
                    Some(
                        s.goal_loop_ran
                            .replace("{ligne}", &ligne.to_string())
                            .replace("{tours}", &tours.to_string()),
                    )
                }
                namsdec_re::explain::Fait::SautToujoursPris { adresse } => Some(
                    s.goal_jump_always
                        .replace("{ligne}", &ligne_de(adresse)?.to_string()),
                ),
                namsdec_re::explain::Fait::SautJamaisPris { adresse } => Some(
                    s.goal_jump_never
                        .replace("{ligne}", &ligne_de(adresse)?.to_string()),
                ),
            })
            .take(2)
            .collect()
    }

    pub(crate) fn line_visits(&self) -> BTreeMap<u32, u64> {
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            return BTreeMap::new();
        };
        let mut par_ligne: BTreeMap<u32, u64> = BTreeMap::new();
        for (addr, count) in emulator.visits() {
            if let Some(line) = state.addr_to_line.get(addr) {
                *par_ligne.entry(*line).or_insert(0) += count;
            }
        }
        par_ligne
    }

    pub(crate) fn remember_registers(&mut self) {
        self.regs_before = self.emulator.as_ref().map(|e| e.cpu().regs.clone());
        self.flags_before = self.emulator.as_ref().map(|e| e.cpu().flags);
        // Et la fenêtre mémoire qu'on regarde : mettre en avant ce qui a
        // changé vaut pour elle autant que pour les registres, et cent
        // vingt-huit octets copiés à chaque pas ne coûtent rien.
        self.mem_before = self
            .emulator
            .as_ref()
            .and_then(|e| e.cpu().mem.peek_bytes(self.mem_addr, MEMORY_WINDOW).ok())
            .map(<[u8]>::to_vec);
    }

    /// Remet la machine dans l'état où le chargement l'avait laissée.
    ///
    /// Reconstruire l'émulateur suffit pour le processeur et la mémoire,
    /// mais ce qui l'entoure gardait les traces de la course précédente :
    /// les diagnostics d'un binaire ouvert, l'onglet resté ouvert sur
    /// eux, le compte de pas, et le listing immobile là où l'exécution
    /// s'était arrêtée. Rien de cela ne décrit plus le programme qu'on
    /// vient de ramener à son point d'entrée, et le bouton passait pour
    /// n'avoir rien fait.
    pub(crate) fn do_reset(&mut self) {
        self.show_input_dialog = false;
        // Les diagnostics d'un binaire ouvert ne viennent pas d'un
        // assemblage : `load_elf` les vide en le chargeant, et les
        // laisser ici ferait de « Réinitialiser » un demi-chargement.
        // Ceux d'un source, eux, désignent des lignes qui n'ont pas
        // bougé : les effacer perdrait ce que nasm avait à dire.
        let from_binary = self
            .assemble_state
            .as_ref()
            .is_some_and(|state| state.from_binary.is_some());
        if from_binary {
            self.diagnostics.clear();
            if self.console_tab == ConsoleTab::Diagnostics {
                self.console_tab = ConsoleTab::Output;
            }
        }
        self.rebuild_emulator();
        self.run_steps_taken = 0;
        // Le listing suit le programme jusqu'à son point d'entrée : le
        // laisser où la course s'était arrêtée montrait du code que plus
        // rien n'allait exécuter.
        self.follow_execution();
        self.status = Status::Idle;
    }

    /// (Re)construit l'émulateur dans l'état du tout début.
    ///
    /// Deux chemins, parce qu'il y a deux sortes de programmes. Celui
    /// qu'on vient d'écrire est posé tel quel, à l'adresse qu'on lui a
    /// donnée, avec une pile vierge : c'est ce qu'on veut en apprenant, et
    /// rien d'autre ne doit bouger sous les pieds. Celui qui vient d'un
    /// fichier, lui, passe par le **chargeur** — segments, relocations,
    /// TLS, pile de processus —, faute de quoi il ne survit pas à ses dix
    /// premières instructions.
    pub(crate) fn rebuild_emulator(&mut self) {
        if let Some(elf) = self.binary_elf() {
            self.load_process(&elf);
            return;
        }
        self.rebuild_from_assembly();
    }

    /// L'ELF du binaire ouvert, quand c'en est un et qu'il est chargeable.
    ///
    /// Relu à chaque reconstruction plutôt que gardé : l'analyse coûte
    /// une milliseconde sur un binaire de trois cent mille octets, et
    /// garder une image déjà relogée obligerait à la dupliquer pour
    /// pouvoir revenir à l'état initial — ce que « Reset » demande.
    pub(crate) fn binary_elf(&self) -> Option<namsdec_asm::elf::Elf> {
        let state = self.assemble_state.as_ref()?;
        state.from_binary.as_ref()?;
        namsdec_asm::elf::parse(&state.output.file).filter(|elf| !elf.segments.is_empty())
    }

    /// Charge un binaire comme le noyau le ferait, et arme l'émulateur
    /// sur l'état que `_start` trouverait.
    pub(crate) fn load_process(&mut self, elf: &namsdec_asm::elf::Elf) {
        let nom = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
            .map(|binaire| binaire.name.clone())
            .filter(|nom| !nom.is_empty())
            .unwrap_or_else(|| "programme".to_owned());

        let arguments = split_args(&self.program_args);
        let environnement: Vec<String> = self
            .program_env
            .lines()
            .map(str::trim)
            .filter(|ligne| !ligne.is_empty())
            .map(str::to_owned)
            .collect();
        let chargé = match loader::load(elf, &nom, &arguments, &environnement, MEM_MAX) {
            Ok(chargé) => chargé,
            Err(loader::LoadError::TooLarge { .. }) => {
                self.status = Status::Error(strings(self.lang).binary_too_large.to_string());
                self.emulator = None;
                return;
            }
            Err(loader::LoadError::NotLoadable) => {
                self.status = Status::Error(strings(self.lang).binary_not_linked.to_owned());
                self.emulator = None;
                return;
            }
        };

        let mut cpu = namsdec_core::Cpu::new(chargé.memory);
        cpu.regs.rip = chargé.entry;
        cpu.regs.set64(Reg64::Rsp, chargé.rsp);
        // La base FS : sans elle, la première fonction protégée lit son
        // canari à une adresse qui n'existe pas.
        cpu.regs.fs_base = chargé.fs_base;

        // Un programme qui vient d'être chargé n'a pas de passé.
        self.regs_before = None;
        self.flags_before = None;
        self.last_step = None;
        // Et la console n'a pas encore paru pour CE programme : le
        // suivant écrira peut-être, et il doit pouvoir l'ouvrir.
        self.console_ouverte_pour_la_sortie = false;
        // Et pas de réponse déjà découverte : l'adresse révélée
        // appartenait au programme d'avant, et deux programmes
        // différents ont des instructions différentes à la même
        // adresse. Sans cela, « Prévoir avant d'avancer » aurait
        // montré la réponse d'une instruction qu'on n'a jamais vue.
        self.prevision_révélée = None;
        self.error_advice = None;
        self.mem_before = None;

        let mut emulator = Emulator::new(cpu);
        emulator.set_heap(chargé.layout.heap.start, chargé.layout.heap.end);
        emulator.set_mmap(chargé.layout.mmap.start, chargé.layout.mmap.end);
        // Les bornes : arriver sur l'une d'elles, c'est appeler la
        // fonction qu'elle remplace. namsdec en fournit une poignée.
        emulator.set_externals(chargé.externals.clone(), chargé.main_return);
        emulator.set_breakpoints(
            self.breakpoint_lines
                .iter()
                .filter_map(|l| {
                    self.assemble_state
                        .as_ref()
                        .and_then(|state| state.output.line_to_addr.get(l).copied())
                })
                .collect(),
        );

        if let Some(state) = self.assemble_state.as_ref() {
            let ctx = ReContext {
                cpu: emulator.cpu(),
                disasm: &state.disasm,
                source: &self.source,
                binary: &state.output.file,
                lang: self.lang.code(),
                names: Some(&self.notes.names),
                patches: &self.patches,
            };
            self.re_registry.on_binary_loaded(&ctx);
        }

        self.emulator = Some(Box::new(emulator));
        // Ce que le chargeur a posé suit le programme : les bornes
        // servent à nommer un appel qui n'aboutira pas, le rapport à dire
        // ce qui a été fait.
        if let Some(binaire) = self
            .assemble_state
            .as_mut()
            .and_then(|state| state.from_binary.as_mut())
        {
            binaire.externals = chargé.externals;
            binaire.load = chargé.report;
        }
        // Les bornes viennent d'être posées : c'est maintenant qu'on sait
        // si ce programme réclamera une entrée.
        self.note_program_io();
        self.sync_watchpoints();
    }

    /// Le chemin d'un programme qu'on vient d'assembler : l'image telle
    /// quelle, à son adresse, dans la fenêtre historique de l'atelier.
    pub(crate) fn rebuild_from_assembly(&mut self) {
        let Some(state) = &self.assemble_state else {
            return;
        };
        let (base, taille, sommet) =
            memory_window(state.output.load_addr, state.output.bytes.len());
        if taille > MEM_MAX {
            self.status = Status::Error(strings(self.lang).binary_too_large.to_string());
            self.emulator = None;
            return;
        }
        let mut mem = namsdec_core::Memory::new(base, taille);
        // `load_addr` plutôt que `BASE_ADDR` : le programme lié dit
        // lui-même où ses sections doivent se trouver, et `.data` n'est
        // pas collée derrière `.text` mais sur la page suivante.
        if mem
            .load(state.output.load_addr, &state.output.bytes)
            .is_err()
        {
            self.status = Status::Error(strings(self.lang).binary_too_large.to_string());
            self.emulator = None;
            return;
        }
        // Les droits : ce qui sépare un programme qui plante proprement
        // d'un programme qui écrit dans son propre code sans que personne
        // ne le dise.
        //
        // Seulement si l'on sait où sont les sections : un binaire brut
        // ouvert depuis le disque n'en déclare aucune, et inventer des
        // droits pour lui reviendrait à refuser ce qu'on ne comprend pas.
        if !state.output.sections.is_empty() {
            for section in &state.output.sections {
                // Une section vide ne projette rien : la déclarer
                // n'ajouterait qu'une ligne sans contenu à la carte.
                if section.size == 0 {
                    continue;
                }
                let perms = namsdec_core::Perms {
                    read: true,
                    write: section.writable,
                    execute: section.executable,
                };
                mem.map(
                    section.addr..section.addr + section.size as u64,
                    perms,
                    region_label(section),
                );
            }
            // La pile : on y écrit, on n'y exécute pas. C'est la
            // protection qui a mis fin aux débordements de tampon des
            // années 90.
            mem.map(
                HEAP_LIMIT..sommet + 0x100,
                namsdec_core::Perms::DATA,
                "stack",
            );
        }

        let mut cpu = namsdec_core::Cpu::new(mem);
        // Le point d'entrée de l'ELF, et non le premier octet chargé : sur
        // un programme qui commence par `.data`, ce sont deux adresses
        // différentes.
        cpu.regs.rip = state.output.entry;
        cpu.regs.set64(Reg64::Rsp, sommet);

        // Un programme qui vient d'être chargé n'a pas de passé : rien à
        // mettre en avant tant qu'il n'a pas avancé d'un pas.
        self.regs_before = None;
        self.flags_before = None;
        self.last_step = None;
        self.console_ouverte_pour_la_sortie = false;
        // Ni de réponse déjà découverte : on recommence, donc on
        // reprévoit. Voir `load_process` pour l'autre moitié.
        self.prevision_révélée = None;
        self.error_advice = None;
        self.mem_before = None;
        let mut emulator = Emulator::new(cpu);
        // Le tas commence après l'image du programme et monte vers la
        // pile : c'est la disposition d'un vrai processus, et c'est elle
        // qui donne un sens à `brk`.
        emulator.set_heap(
            state.output.load_addr + state.output.bytes.len() as u64,
            HEAP_LIMIT,
        );
        emulator.set_breakpoints(
            self.breakpoint_lines
                .iter()
                .filter_map(|l| state.output.line_to_addr.get(l).copied())
                .collect(),
        );

        let ctx = ReContext {
            cpu: emulator.cpu(),
            disasm: &state.disasm,
            source: &self.source,
            binary: &state.output.file,
            lang: self.lang.code(),
            names: Some(&self.notes.names),
            patches: &self.patches,
        };
        self.re_registry.on_binary_loaded(&ctx);

        self.emulator = Some(Box::new(emulator));
        // Les zones surveillées survivent au réassemblage : on travaille
        // souvent sur la même adresse en corrigeant son programme.
        self.sync_watchpoints();
    }

    /// Pousse les zones surveillées dans le moteur, ce qui les arme sur
    /// l'état courant.
    pub(crate) fn sync_watchpoints(&mut self) {
        let points = self.watchpoints.clone();
        // Les registres suivent le même chemin : la liste vit dans
        // l'application, le moteur la reçoit et l'arme sur l'état courant.
        let registers: Vec<(&'static str, Reg64)> = REGISTERS
            .iter()
            .filter(|(name, _)| self.watched_registers.contains(name))
            .copied()
            .collect();
        if let Some(emulator) = &mut self.emulator {
            emulator.set_watchpoints(&points);
            emulator.set_watched_registers(&registers);
        }
    }

    /// Ce point d'arrêt doit-il arrêter, cette fois-ci ?
    ///
    /// Sans condition, oui, toujours. Avec une condition qu'on ne sait
    /// pas lire, oui aussi : un point d'arrêt qui ne s'arrête jamais à
    /// cause d'une faute de frappe est pire qu'un point d'arrêt qui
    /// s'arrête trop.
    pub(crate) fn breakpoint_holds(&self, addr: u64) -> bool {
        let Some(state) = &self.assemble_state else {
            return true;
        };
        let Some(line) = state.addr_to_line.get(&addr) else {
            return true;
        };
        let Some(texte) = self.breakpoint_conditions.get(line) else {
            return true;
        };
        let Some(condition) = condition::parse(texte) else {
            return true;
        };
        let Some(emulator) = &self.emulator else {
            return true;
        };
        let cpu = emulator.cpu();
        condition.holds(&cpu.regs, Some(&PeekMemory(&cpu.mem)))
    }

    pub(crate) fn sync_breakpoints(&mut self) {
        let Some(state) = &self.assemble_state else {
            return;
        };
        let addrs: BTreeSet<u64> = self
            .breakpoint_lines
            .iter()
            .filter_map(|l| state.output.line_to_addr.get(l).copied())
            .collect();
        if let Some(emulator) = &mut self.emulator {
            emulator.set_breakpoints(addrs);
        }
    }

    /// Relève ce que l'exécution vient de toucher de simplifié, et le
    /// dit — une fois chacune, au moment où ça se produit.
    ///
    /// Au moment où ça se produit, et pas au démarrage : un
    /// avertissement lu avant d'avoir exécuté quoi que ce soit ne se
    /// rattache à rien, et c'est comme ça qu'on apprend à ne plus les
    /// lire. Celui-ci paraît juste après le `fsin` ou le `fstp tword`
    /// qui l'a provoqué, et le panneau des diagnostics le garde.
    fn recueille_les_simplifications(&mut self) {
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        let nouvelles = emulator.pending_simplifications();
        if nouvelles.is_empty() {
            return;
        }
        // La barre d'état n'en montre qu'une — la dernière rencontrée.
        // Une barre qui empile sept phrases n'en montre aucune ; le
        // panneau des diagnostics, lui, les tient toutes, et il les lit
        // dans l'émulateur plutôt que d'en garder une copie : la liste
        // suit alors l'onglet, et un autre programme n'hérite pas des
        // approximations du précédent.
        if let Some(derniere) = nouvelles.last() {
            self.status = Status::Simplified(*derniere);
        }
    }

    /// La console paraît dès que le programme écrit quelque chose.
    ///
    /// **C'est ce qui rend tenable une console fermée au départ.** Elle
    /// l'est sur une installation neuve : avant d'avoir assemblé, elle
    /// n'a rien à dire et occupe soixante-douze pixels pour l'annoncer.
    /// Mais un programme qui affiche « salut » doit l'afficher quelque
    /// part — sans cela, on croit qu'il n'a rien fait.
    ///
    /// Une fois par programme, et pas à chaque octet : qui referme la
    /// console en pleine course a choisi, et on ne la lui rouvre pas
    /// sous les doigts. Le drapeau se remet à zéro au chargement, comme
    /// le reste du passé.
    ///
    /// C'est la même règle que les diagnostics de nasm
    /// (`show_diagnostics_if_any`), et elle existe depuis plus
    /// longtemps : une erreur d'assemblage ouvre déjà le panneau. On
    /// étend à la SORTIE ce qui valait pour les erreurs.
    fn montrer_la_console_si_le_programme_ecrit(&mut self) {
        if self.console_ouverte_pour_la_sortie {
            return;
        }
        let a_écrit = self
            .emulator
            .as_ref()
            .is_some_and(|emulator| !emulator.output().is_empty());
        if !a_écrit {
            return;
        }
        self.console_ouverte_pour_la_sortie = true;
        if !self.show_console {
            self.show_console = true;
            self.console_tab = ConsoleTab::Output;
        }
    }

    pub(crate) fn notify_step(&mut self) {
        self.montrer_la_console_si_le_programme_ecrit();
        // Le battement d'orange part d'ici : notify_step est le point par
        // où passent tous les pas (un pas, un Run qui s'arrête, un retour
        // en arrière), et c'est à cet instant que l'état nouveau devient
        // visible. Ce qui a changé, le diff le dit déjà ; quand, c'est
        // cet horodatage.
        self.values_changed_at = Some(std::time::Instant::now());
        self.recueille_les_simplifications();
        // L'éditeur suit l'exécution : au prochain rendu, il ramène son
        // défilement horizontal à zéro, pour que les instructions et la
        // ligne courante ne restent pas sorties à gauche.
        self.editor_follow = 8;
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            return;
        };
        let ctx = ReContext {
            cpu: emulator.cpu(),
            disasm: &state.disasm,
            source: &self.source,
            binary: &state.output.file,
            lang: self.lang.code(),
            names: Some(&self.notes.names),
            patches: &self.patches,
        };
        self.re_registry.on_step(&ctx);
    }

    /// Retient l'instruction que le pas qui commence va exécuter.
    ///
    /// Prise *avant* le pas : après, RIP a bougé, et l'instruction qu'on
    /// vient de faire n'est plus sous le pointeur. C'est elle qui donne
    /// son sens à ce qui a changé — « RCX 0x5 → 0x4 » ne veut rien dire
    /// sans le `dec rcx` qui l'a fait.
    pub(crate) fn remember_step(&mut self) {
        let lang = self.lang.code();
        self.last_step = self.emulator.as_ref().and_then(|emulator| {
            let cpu = emulator.cpu();
            let rip = cpu.regs.rip;
            let window = cpu.mem.peek_bytes(rip, 16).ok()?;
            let instr = namsdec_disasm::decode_one(window, rip)?;
            let texte = namsdec_disasm::format_one(&instr);
            // CE QUE L'INSTRUCTION ANNONÇAIT, pris ici et nulle part
            // ailleurs : c'est exactement ce que « prévoir avant
            // d'avancer » avait retenu, et après le pas l'instruction
            // n'est plus sous le pointeur. Le calculer plus tard
            // demanderait de la redécoder en espérant que les octets
            // n'ont pas bougé — ce qu'un programme qui se patche
            // lui-même démentirait.
            let annonce =
                crate::prevision::annonce(&texte, &namsdec_disasm::pseudo::of(&instr, lang));
            Some(crate::DernierPas {
                addr: rip,
                texte,
                annonce,
            })
        });
    }

    /// Rend compte d'une erreur d'exécution : le message et, quand il y
    /// en a un, le conseil qui va avec.
    pub(crate) fn report_error(&mut self, e: &namsdec_emu::EmuError) {
        self.error_advice = errors::advice(self.lang, e).map(str::to_owned);
        self.status = Status::Error(errors::emu(self.lang, e));
    }

    /// Avant d'exécuter quoi que ce soit : si nasm travaille encore,
    /// on l'attend.
    ///
    /// Demander un pas pendant un assemblage ne veut pas dire « ignore
    /// ma demande », cela veut dire « quand il aura fini ». Sans cela,
    /// frapper Step aussitôt après Ctrl+Maj+B ne faisait rien du tout
    /// — et rien du tout est ce qui ressemble le plus à une panne.
    pub(crate) fn assurer_le_programme(&mut self) {
        if self.assembling() {
            self.attendre_l_assemblage();
        }
    }

    pub(crate) fn do_step(&mut self) {
        self.assurer_le_programme();
        self.remember_registers();
        self.remember_step();
        // Le conseil vaut pour l'erreur qui vient de se produire : au pas
        // suivant, il parlerait d'autre chose.
        self.error_advice = None;
        self.follow_execution();
        // Avancer d'un pas, c'est reprendre la main : un Run suspendu
        // faute d'entrée ne doit pas repartir tout seul à la saisie
        // suivante, sans quoi la ligne tapée pour ce pas-ci relancerait
        // le programme jusqu'au bout.
        self.resume_run_after_input = false;
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match emulator.step() {
            Ok(StepResult::Halted) => {
                self.status = halted_status(emulator.as_ref());
                self.back_to_command_mode();
            }
            Ok(StepResult::NeedsInput) => self.wait_for_input(),
            Ok(StepResult::Continued) => {}
            Err(e) => self.report_error(&e),
        }
        self.notify_step();
    }

    /// Démarre un Run — ne bloque plus le thread UI jusqu'à la fin :
    /// `tick_run` avance par petits paquets, une frame à la fois, pour que
    /// Stop reste réactif.
    pub(crate) fn do_run(&mut self) {
        self.assurer_le_programme();
        if self.emulator.is_none() {
            return;
        }
        // La boîte de saisie parlait de la course précédente : elle se
        // rouvrira d'elle-même si celle-ci demande quelque chose.
        self.show_input_dialog = false;
        // Une course n'a pas « un » dernier pas : le bandeau se tait
        // plutôt que de montrer la première instruction d'un millier.
        self.last_step = None;
        self.error_advice = None;
        // Photographie prise au départ : à l'arrêt suivant — un point
        // d'arrêt, un `hlt` — le bandeau montre tout ce que la course a
        // changé, et non le seul dernier pas.
        self.remember_registers();
        self.resume_run_after_input = false;
        self.running = true;
        self.run_steps_taken = 0;
        self.status = Status::Running;
    }

    /// Exécute jusqu'à la ligne du curseur.
    ///
    /// Un point d'arrêt pour un seul voyage : le geste qu'on fait vingt
    /// fois par heure en apprenant — « va jusque-là et montre-moi » —
    /// demandait jusqu'ici de poser un point, de courir, puis de penser à
    /// le retirer.
    pub(crate) fn do_run_to_cursor(&mut self) {
        self.last_step = None;
        self.error_advice = None;
        let Some(state) = &self.assemble_state else {
            return;
        };
        // La ligne du curseur, ou la première en dessous qui produise du
        // code : on pose souvent le curseur sur une ligne vide ou sur un
        // commentaire, et « aller là » veut alors dire « aller à ce qui
        // suit ».
        let from = self.cursor_pos.0 as u32;
        let Some((_, &addr)) = state.output.line_to_addr.range(from..).next() else {
            return;
        };
        let mut points = self
            .breakpoint_lines
            .iter()
            .filter_map(|l| state.output.line_to_addr.get(l).copied())
            .collect::<BTreeSet<u64>>();
        points.insert(addr);
        if let Some(emulator) = &mut self.emulator {
            emulator.set_breakpoints(points);
        }
        self.run_to = Some(addr);
        self.do_run();
    }

    /// Court jusqu'à la prochaine instruction de ce genre.
    ///
    /// Les points d'arrêt sont posés d'avance sur toutes les
    /// instructions visées — un pas à pas jusqu'à la bonne aurait figé
    /// l'interface sur une boucle un peu longue. L'instruction courante
    /// est exclue, sans quoi « au prochain appel » n'irait nulle part
    /// quand on est déjà sur un `call`.
    ///
    /// Rien à viser n'est pas une erreur : le programme ne contient
    /// simplement pas ce qu'on cherche, et le dire vaut mieux que de le
    /// laisser courir jusqu'à la fin.
    pub(crate) fn do_run_until(&mut self, kind: UntilKind) {
        self.last_step = None;
        self.error_advice = None;
        let Some(state) = &self.assemble_state else {
            return;
        };
        let ici = self.emulator.as_ref().map(|e| e.cpu().regs.rip);
        let mut points: BTreeSet<u64> = state
            .disasm
            .iter()
            .filter(|instruction| Some(instruction.address) != ici)
            .filter_map(|instruction| {
                namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
                    .filter(|decoded| kind.matches(decoded))
                    .map(|_| instruction.address)
            })
            .collect();
        if points.is_empty() {
            self.status = Status::Note(match kind {
                UntilKind::Call => |s: &Strings| s.run_until_no_call,
                UntilKind::Return => |s: &Strings| s.run_until_no_return,
                UntilKind::Syscall => |s: &Strings| s.run_until_no_syscall,
            });
            return;
        }
        // Les points de l'utilisateur restent armés : on court vers le
        // prochain appel, mais on s'arrête quand même là où il l'a
        // demandé.
        points.extend(
            self.breakpoint_lines
                .iter()
                .filter_map(|l| state.output.line_to_addr.get(l).copied()),
        );
        if let Some(emulator) = &mut self.emulator {
            emulator.set_breakpoints(points);
        }
        self.run_until = Some(kind);
        self.do_run();
    }

    /// À appeler une fois par frame. N'avance que si un Run est en cours.
    pub(crate) fn tick_run(&mut self, ctx: &egui::Context) {
        if !self.running {
            // La course finie, le point d'arrêt d'un seul voyage s'en va
            // avec elle : il n'appartient pas à l'utilisateur, et le
            // laisser là ferait s'arrêter la course suivante à un endroit
            // que personne n'a demandé.
            if self.run_to.take().is_some() | self.run_until.take().is_some() {
                self.sync_breakpoints();
            }
            return;
        }
        // Un point d'arrêt dont la condition est fausse ne doit pas
        // coûter une frame : la course reprend tout de suite, dans le
        // budget de pas de cette frame-ci. Sans cela, traverser une
        // boucle de cinq cents tours demanderait cinq cents frames —
        // huit secondes à ne rien voir bouger.
        let mut budget = RUN_BATCH_STEPS;
        let arrêt = loop {
            let Some(emulator) = &mut self.emulator else {
                self.running = false;
                return;
            };
            let avant = emulator.executed();
            let arrêt = emulator.run(budget);
            let consommés = self
                .emulator
                .as_ref()
                .map(|e| e.executed().saturating_sub(avant))
                .unwrap_or(0);
            budget = budget.saturating_sub(consommés.min(u64::from(budget)) as u32);
            match &arrêt {
                Ok(RunStop::Breakpoint(addr)) if !self.breakpoint_holds(*addr) && budget > 0 => {
                    continue;
                }
                _ => break arrêt,
            }
        };
        let Some(emulator) = &mut self.emulator else {
            self.running = false;
            return;
        };
        match arrêt {
            Ok(RunStop::Halted) => {
                self.running = false;
                self.status = halted_status(emulator.as_ref());
                self.back_to_command_mode();
            }
            Ok(RunStop::Breakpoint(addr)) => {
                // La condition décide. Fausse, la course reprend sans
                // que personne n'ait à cliquer : c'est tout l'objet d'un
                // point d'arrêt conditionnel dans une boucle de mille
                // tours.
                if self.breakpoint_holds(addr) {
                    self.running = false;
                    self.status = Status::Breakpoint(addr);
                }
            }
            Ok(RunStop::Watchpoint { addr, old, new }) => {
                self.running = false;
                self.status = Status::Watchpoint { addr, old, new };
                self.back_to_command_mode();
            }
            Ok(RunStop::RegisterChanged { register, old, new }) => {
                self.running = false;
                self.status = Status::RegisterChanged { register, old, new };
                self.back_to_command_mode();
            }
            Ok(RunStop::NeedsInput) => {
                // Suspendu, pas arrêté : l'exécution redémarrera d'
                // elle-même dès que l'utilisateur aura tapé quelque chose.
                self.running = false;
                self.resume_run_after_input = true;
                self.wait_for_input();
            }
            Ok(RunStop::StepLimit) => {
                self.run_steps_taken += RUN_BATCH_STEPS as u64;
                if self.run_steps_taken >= RUN_TOTAL_STEPS_CAP {
                    self.running = false;
                    self.status = Status::StepLimit;
                }
            }
            Err(e) => {
                self.running = false;
                self.report_error(&e);
            }
        }
        self.notify_step();
        if self.running {
            // egui ne redessine que sur un événement d'entrée par défaut :
            // il faut forcer des frames tant qu'un Run est en cours.
            ctx.request_repaint();
        }
    }

    /// Interrompt un Run en cours. Prend effet au prochain batch (latence
    /// bornée par RUN_BATCH_STEPS, pas instantanée), sans exiger que
    /// `Emulator` devienne `Send`/threadable.
    pub(crate) fn do_stop(&mut self) {
        // Le drapeau tombe même quand rien ne tourne : un Run suspendu
        // faute d'entrée a `running == false` et attend pourtant de
        // repartir tout seul. Sans ça, Stop le laissait armé.
        self.resume_run_after_input = false;
        // ET L'ASSEMBLAGE, depuis qu'il tourne en fond. Sans cela, le
        // seul bouton qui dise « arrête » ne pouvait rien contre le
        // seul travail dont on veuille vraiment sortir : nasm occupé
        // dix secondes sur un `times` démesuré.
        if self.assembling() {
            self.cancel_assemble();
            self.status = Status::Note(|s| s.assemble_cancelled);
            return;
        }
        if self.running {
            self.running = false;
            self.status = Status::Idle;
        }
    }

    /// Défait le dernier pas.
    ///
    /// Le bandeau montre alors ce que ce pas avait changé : la
    /// photographie est prise *avant* de reculer, si bien que la mise en
    /// avant désigne exactement ce qu'on vient d'annuler.
    pub(crate) fn do_step_back(&mut self) {
        self.follow_execution();
        // Le pas qu'on défait n'est plus le dernier pas fait.
        self.last_step = None;
        // Un Run suspendu ne repart pas tout seul après un retour en
        // arrière : c'est l'utilisateur qui tient la main, comme pour un
        // pas ordinaire.
        self.resume_run_after_input = false;
        self.running = false;
        self.remember_registers();
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        if emulator.step_back() {
            self.status = Status::SteppedBack;
            self.back_to_command_mode();
            self.notify_step();
        } else {
            // Rien à défaire : on le dit plutôt que de ne rien faire.
            self.status = Status::NothingToUndo;
            self.regs_before = None;
            self.flags_before = None;
            self.last_step = None;
            self.error_advice = None;
            self.mem_before = None;
        }
    }

    pub(crate) fn do_step_over(&mut self) {
        self.assurer_le_programme();
        self.follow_execution();
        self.remember_registers();
        self.remember_step();
        // Même raison que dans `do_step`.
        self.resume_run_after_input = false;
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match emulator.step_over(MAX_RUN_STEPS) {
            Ok(StepOverResult::Halted) => {
                self.status = halted_status(emulator.as_ref());
                self.back_to_command_mode();
            }
            Ok(StepOverResult::Breakpoint(addr)) => self.status = Status::Breakpoint(addr),
            Ok(StepOverResult::Watchpoint { addr, old, new }) => {
                self.status = Status::Watchpoint { addr, old, new }
            }
            Ok(StepOverResult::StepLimit) => self.status = Status::StepLimit,
            Ok(StepOverResult::NeedsInput) => self.wait_for_input(),
            Ok(StepOverResult::Stepped | StepOverResult::SteppedOverCall) => {}
            Err(e) => self.report_error(&e),
        }
        self.notify_step();
    }

    /// Point d'entrée unique pour tout ce qui déclenche une action —
    /// raccourci clavier ou clic de bouton/menu, pour ne jamais avoir deux
    /// implémentations de "ce que fait Assembler" qui divergent.
    /// Le programme chargé est-il allé jusqu'au bout ? `halted` couvre les
    /// deux fins possibles : `exit`, et la machine arrêtée par `hlt`.
    pub(crate) fn program_finished(&self) -> bool {
        self.emulator
            .as_ref()
            .is_some_and(|emulator| emulator.cpu().halted)
    }

    /// Le programme réclame de quoi lire.
    ///
    /// Chaque nouvelle attente rend son focus au champ, même si la
    /// précédente s'était terminée par un Échap : ce qu'on a refusé une
    /// fois, on ne le refuse pas pour toujours.
    pub(crate) fn wait_for_input(&mut self) {
        self.status = Status::WaitingForInput;
        self.console_input_released = false;
    }

    /// Assembler a-t-il une chance d'aboutir ?
    ///
    /// Le bouton se grise plutôt que d'échouer : un listing de binaire
    /// sera refusé à tous les coups, et rien ne l'annonçait avant le
    /// clic. Le raccourci et la palette, eux, passent toujours par
    /// `do_assemble` — refuser en disant pourquoi vaut mieux que ne rien
    /// faire du tout quand le geste ne vient pas d'un bouton qu'on voit.
    pub(crate) fn can_assemble(&self) -> bool {
        !self.buffer_came_from_a_binary()
    }

    /// Les commandes d'exécution ont-elles encore un sens ?
    ///
    /// Il faut un programme chargé, et qu'il lui reste quelque chose à
    /// faire : relancer Step Into sur un programme terminé ne peut plus
    /// rien produire, et laisser le bouton actif laisse croire le
    /// contraire. `Réinitialiser` reste disponible — c'est précisément ce
    /// qu'on veut faire ensuite.
    pub(crate) fn can_execute(&self) -> bool {
        self.emulator.is_some() && !self.program_finished()
    }
}
