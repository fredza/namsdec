//! Interpréteur x86-64 "maison" : le décodage octets -> instruction
//! structurée est délégué à `namsdec-disasm` (donc à `iced-x86`), mais toute
//! la sémantique d'exécution (effets sur registres/flags/mémoire) est
//! écrite ici, à la main, pour rester transparente et pédagogique.

mod regmap;

use std::collections::{BTreeSet, VecDeque};

use iced_x86::{ConditionCode, Instruction, Mnemonic, OpKind, Register};
use namsdec_core::{
    Cpu, Memory, RFlags, Reg64, Width, add_with_flags, logic_flags, sub_with_flags,
};
use thiserror::Error;

use regmap::map_register;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Là où une opérande n'a pas pu être traitée — le sens de la phrase
/// change en français (« opérande source/destination »), la variante le
/// garde donc plutôt que de figer une formulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandRole {
    Destination,
    Read,
    Write,
}

/// Erreurs d'exécution. `Display` est en anglais et les valeurs restent
/// accessibles : c'est `namsdec-gui` qui rend le message dans la langue de
/// l'utilisateur (voir `MemError` pour le pourquoi).
///
/// « Non supporté » désigne toujours l'interpréteur *courant* : un autre
/// moteur d'exécution branché derrière `Backend` peut parfaitement
/// connaître l'instruction que celui-ci refuse, d'où le nom du backend
/// dans le message.
#[derive(Debug, Error)]
pub enum EmuError {
    #[error("{backend}: unsupported instruction: {mnemonic}")]
    UnsupportedInstruction {
        backend: &'static str,
        mnemonic: String,
    },
    #[error("{backend}: unsupported register: {register}")]
    UnsupportedRegister {
        backend: &'static str,
        register: String,
    },
    #[error("{backend}: unsupported {role:?} operand: {kind}")]
    UnsupportedOperand {
        backend: &'static str,
        role: OperandRole,
        kind: String,
    },
    #[error("{backend}: unsupported memory operand size: {bytes} bytes")]
    UnsupportedOperandSize { backend: &'static str, bytes: usize },
    #[error("{backend}: unsupported syscall: {number}")]
    UnsupportedSyscall { backend: &'static str, number: u64 },
    #[error("memory error: {0}")]
    Mem(#[from] namsdec_core::MemError),
    #[error("invalid instruction at {addr:#x}")]
    InvalidInstruction { addr: u64 },
    /// Ce que le processeur signale par l'exception #DE : une division
    /// par zéro, ou un quotient trop grand pour le registre qui doit le
    /// recevoir. Deux variantes distinctes parce que ce ne sont pas la
    /// même erreur de programmation.
    #[error("division by zero at {addr:#x}")]
    DivideByZero { addr: u64 },
    #[error("quotient too large at {addr:#x}")]
    DivideOverflow { addr: u64 },
}

/// Une zone de mémoire surveillée : on s'arrête quand elle change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watchpoint {
    pub addr: u64,
    /// Un à huit octets — la taille de ce qu'on regarde d'habitude : un
    /// octet, un mot, un pointeur.
    pub len: usize,
    /// La valeur vue la dernière fois. C'est la comparaison avec elle qui
    /// déclenche l'arrêt, et non l'écriture : un `mov [x], 0` sur une
    /// case déjà nulle n'apprend rien et n'arrête rien.
    previous: u64,
}

impl Watchpoint {
    /// La valeur retenue au dernier armement.
    pub fn value(&self) -> u64 {
        self.previous
    }
}

/// Un appel en cours : ce qu'il faut pour remonter la chaîne.
///
/// L'émulateur *voit* les `call` et les `ret` passer, il n'a donc pas à
/// deviner les cadres en suivant RBP comme le fait un débogueur sans
/// symboles — ce qui échoue justement sur le code qu'on écrit en
/// apprenant, où personne ne monte de cadre de pile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// L'adresse de l'instruction `call` elle-même : c'est elle qui se
    /// traduit en « appelé depuis la ligne … ».
    pub call_site: u64,
    /// Où l'appel est parti — le début de la sous-routine.
    pub entry: u64,
    /// Où il reviendra.
    pub return_addr: u64,
    /// RSP juste après l'empilement de l'adresse de retour.
    pub rsp: u64,
}

/// Ce qu'un pas a fait à la pile des appels, pour pouvoir le défaire.
#[derive(Clone, Copy, Debug)]
enum FrameChange {
    /// Ni `call` ni `ret` : rien à défaire.
    None,
    /// Un `call` : le cadre empilé est à retirer.
    Pushed,
    /// Un `ret` : le cadre dépilé est à remettre.
    Popped(Frame),
}

/// Ce qu'il faut retenir d'un pas pour pouvoir le défaire.
///
/// Un instantané complet de la machine coûterait la mémoire entière à
/// chaque instruction ; on ne garde donc que ce qui est petit (registres,
/// flags, état d'arrêt) et, pour la mémoire, les seuls octets que le pas
/// s'apprête à écraser. Un pas ordinaire n'en touche aucun.
#[derive(Clone, Debug)]
struct Undo {
    regs: namsdec_core::Registers,
    flags: RFlags,
    halted: bool,
    exit_code: Option<i64>,
    /// Longueur de la sortie avant le pas : ce qu'il a écrit se tronque.
    output_len: usize,
    /// Les octets d'entrée qu'un `read` a consommés, à remettre en tête.
    consumed_input: Vec<u8>,
    input_closed: bool,
    /// Les octets écrasés, avec leur adresse, dans l'ordre des écritures.
    memory: Vec<(u64, Vec<u8>)>,
    /// Ce que le pas a fait à la pile des appels.
    frames: FrameChange,
}

/// Combien de pas on peut défaire. Au-delà, les plus anciens sont
/// oubliés : revenir mille instructions en arrière n'a pas de sens
/// pédagogique, et la mémoire n'est pas extensible.
const HISTORY_LIMIT: usize = 5_000;

/// Le sens d'un décalage, et s'il recopie le bit de signe.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ShiftKind {
    Left,
    Right,
    /// `sar` : décalage à droite qui recopie le bit de signe, donc une
    /// division entière par une puissance de deux.
    Arithmetic,
}

/// Lit `len` octets (1 à 8) en petit-boutien. Hors de la fenêtre
/// simulée, rend zéro : une zone surveillée qu'on ne peut pas lire ne
/// change jamais, plutôt que de faire échouer l'exécution.
fn read_bytes(mem: &namsdec_core::Memory, addr: u64, len: usize) -> u64 {
    let mut value = 0u64;
    for i in (0..len).rev() {
        let byte = mem.read_u8(addr.wrapping_add(i as u64)).unwrap_or(0);
        value = (value << 8) | u64::from(byte);
    }
    value
}

/// Étend `value`, large de `w`, sur 64 bits en recopiant son bit de signe.
fn sign_extend(value: u64, w: Width) -> u64 {
    let bits = w.bits();
    if bits >= 64 {
        return value;
    }
    let shift = 64 - bits;
    (((value << shift) as i64) >> shift) as u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Continued,
    Halted,
    /// Le programme a demandé à lire son entrée standard, et il n'y a rien
    /// à lui donner. L'instruction n'a **pas** été exécutée et RIP est
    /// resté dessus : dès que l'entrée arrive, le même `syscall` repart
    /// comme si de rien n'était.
    NeedsInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStop {
    Halted,
    Breakpoint(u64),
    /// Un point de surveillance a vu la mémoire changer sous lui.
    ///
    /// Là où un point d'arrêt demande « arrête-toi *là* », celui-ci
    /// demande « arrête-toi quand *ça* change » — et l'on ne sait pas
    /// d'avance quelle instruction le fera. C'est ce qui le rend utile,
    /// et coûteux dans un vrai débogueur ; ici, l'interpréteur passe déjà
    /// par un seul endroit pour écrire en mémoire.
    Watchpoint {
        addr: u64,
        old: u64,
        new: u64,
    },
    StepLimit,
    /// Suspendu en attendant l'entrée standard (voir
    /// `StepResult::NeedsInput`). L'exécution reprend d'elle-même une fois
    /// l'entrée fournie.
    NeedsInput,
}

/// Résultat d'un `step_over` : distinct de `StepResult`/`RunStop` parce que
/// "j'ai enjambé un appel entier" n'est ni un simple pas, ni une fin de
/// `run()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOverResult {
    /// Comme `RunStop::Watchpoint` : la sous-routine enjambée a touché à
    /// une zone surveillée.
    Watchpoint {
        addr: u64,
        old: u64,
        new: u64,
    },
    /// L'instruction courante n'était pas un `call` : un pas normal a été
    /// fait, exactement comme `step()`.
    Stepped,
    /// C'était un `call` : toute la sous-routine a été exécutée d'un bloc,
    /// RIP est revenu juste après l'appel.
    SteppedOverCall,
    Halted,
    /// Un point d'arrêt a été atteint à l'intérieur de la sous-routine.
    Breakpoint(u64),
    StepLimit,
    /// Suspendu en attendant l'entrée standard (voir
    /// `StepResult::NeedsInput`).
    NeedsInput,
}

/// Un moteur d'exécution x86-64, vu par l'interface.
///
/// L'interpréteur maison (`Emulator`) en est le premier, et le seul pour
/// l'instant : il exécute pas à pas et *explique*, ce qui est tout l'objet
/// pédagogique de namsdec. Un moteur de production (Unicorn, par exemple)
/// couvre en revanche bien plus d'instructions et va bien plus vite — deux
/// qualités qui n'ont de sens qu'à côté de la première, pas à sa place.
/// D'où ce trait plutôt qu'un remplacement : le jour où un second moteur
/// arrive, il se branche ici et l'interface ne change pas d'une ligne.
///
/// `cpu()` fait partie du contrat parce que toute la barre latérale
/// (registres, flags, pile, mémoire) lit cet état-là. Un moteur qui tient
/// son état ailleurs — dans un processus, dans une VM — le recopie donc
/// dans ce `Cpu` après chaque pas : c'est la vue que l'interface sait
/// afficher, et c'est le prix d'entrée pour s'y brancher.
pub trait Backend {
    /// Nom court du moteur, tel qu'il apparaît dans les messages
    /// d'erreur : « non supporté » ne veut rien dire sans dire *par qui*.
    fn name(&self) -> &'static str;

    fn cpu(&self) -> &Cpu;
    fn cpu_mut(&mut self) -> &mut Cpu;

    fn breakpoints(&self) -> &BTreeSet<u64>;
    fn set_breakpoints(&mut self, breakpoints: BTreeSet<u64>);

    /// Les zones à surveiller — adresse et longueur (1 à 8 octets).
    /// Poser un point de surveillance l'arme sur l'état courant.
    fn set_watchpoints(&mut self, points: &[(u64, usize)]);

    /// Les zones surveillées, avec la valeur retenue.
    fn watchpoints(&self) -> &[Watchpoint];

    fn step(&mut self) -> Result<StepResult, EmuError>;
    fn step_over(&mut self, max_steps: u32) -> Result<StepOverResult, EmuError>;
    fn run(&mut self, max_steps: u32) -> Result<RunStop, EmuError>;

    /// Défait le dernier pas, s'il en reste un à défaire.
    ///
    /// Un moteur qui ne sait pas revenir en arrière rend `false` sans
    /// rien faire : c'est à l'interface de griser son bouton, pas au
    /// programme de se retrouver dans un état à moitié restauré.
    fn step_back(&mut self) -> bool;

    /// Reste-t-il un pas à défaire ?
    fn can_step_back(&self) -> bool;

    /// Les appels en cours, du plus ancien au plus récent. Vide quand
    /// l'exécution est au niveau du programme principal.
    fn frames(&self) -> &[Frame];

    /// Ce que le programme a écrit sur sa sortie standard depuis le
    /// démarrage.
    fn output(&self) -> &[u8];

    /// Ajoute des octets à l'entrée standard du programme.
    fn provide_input(&mut self, bytes: &[u8]);

    /// Signale la fin de l'entrée : les `read` suivants rendront 0 (fin de
    /// fichier) au lieu d'attendre. L'équivalent d'un Ctrl+D.
    fn close_input(&mut self);

    /// Reste-t-il de l'entrée non lue ? Sert à dire à l'utilisateur ce que
    /// le programme va trouver.
    fn pending_input(&self) -> usize;

    /// Le code rendu par `exit`, si le programme s'est terminé de
    /// lui-même. `None` tant qu'il tourne, ou s'il s'est arrêté sur `hlt`
    /// — qui arrête la machine sans rien rendre.
    fn exit_code(&self) -> Option<i64>;
}

pub struct Emulator {
    pub cpu: Cpu,
    pub breakpoints: BTreeSet<u64>,
    /// Ce que le programme a écrit via `write` — des octets bruts, pas du
    /// texte : un programme peut parfaitement écrire ce qu'il veut sur sa
    /// sortie, y compris de l'UTF-8 incomplet ou du binaire, et ce n'est
    /// pas à l'émulateur d'en décider.
    output: Vec<u8>,
    /// Ce que l'utilisateur a fourni et que le programme n'a pas encore
    /// lu. Une file : `read` consomme par le début, l'interface ajoute
    /// par la fin.
    input: VecDeque<u8>,
    /// L'entrée est-elle terminée ? Tant qu'elle ne l'est pas, un `read`
    /// sur un tampon vide *attend* au lieu de rendre une fin de fichier —
    /// c'est ce que fait un vrai terminal, et c'est ce qui distingue
    /// « rien pour l'instant » de « plus rien à lire ».
    input_closed: bool,
    exit_code: Option<i64>,
    /// De quoi défaire les derniers pas — voir `Undo` et `step_back`.
    history: VecDeque<Undo>,
    /// Les appels en cours, du plus ancien au plus récent.
    frames: Vec<Frame>,
    /// Les zones surveillées.
    watchpoints: Vec<Watchpoint>,
}

impl Backend for Emulator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    fn breakpoints(&self) -> &BTreeSet<u64> {
        &self.breakpoints
    }

    fn set_watchpoints(&mut self, points: &[(u64, usize)]) {
        Emulator::set_watchpoints(self, points);
    }

    fn watchpoints(&self) -> &[Watchpoint] {
        Emulator::watchpoints(self)
    }

    fn set_breakpoints(&mut self, breakpoints: BTreeSet<u64>) {
        self.breakpoints = breakpoints;
    }

    fn step(&mut self) -> Result<StepResult, EmuError> {
        Emulator::step(self)
    }

    fn step_over(&mut self, max_steps: u32) -> Result<StepOverResult, EmuError> {
        Emulator::step_over(self, max_steps)
    }

    fn run(&mut self, max_steps: u32) -> Result<RunStop, EmuError> {
        Emulator::run(self, max_steps)
    }

    fn step_back(&mut self) -> bool {
        Emulator::step_back(self)
    }

    fn can_step_back(&self) -> bool {
        Emulator::can_step_back(self)
    }

    fn frames(&self) -> &[Frame] {
        Emulator::frames(self)
    }

    fn output(&self) -> &[u8] {
        &self.output
    }

    fn provide_input(&mut self, bytes: &[u8]) {
        self.input.extend(bytes);
    }

    fn close_input(&mut self) {
        self.input_closed = true;
    }

    fn pending_input(&self) -> usize {
        self.input.len()
    }

    fn exit_code(&self) -> Option<i64> {
        self.exit_code
    }
}

impl Emulator {
    /// Le nom de ce moteur-ci dans les messages d'erreur. Un nom propre,
    /// sans mot traduisible autour : c'est l'interface qui l'enchâsse dans
    /// une phrase française ou anglaise.
    pub const NAME: &'static str = "namsdec";

    pub fn new(cpu: Cpu) -> Self {
        Self {
            cpu,
            breakpoints: BTreeSet::new(),
            output: Vec::new(),
            input: VecDeque::new(),
            input_closed: false,
            exit_code: None,
            history: VecDeque::new(),
            frames: Vec::new(),
            watchpoints: Vec::new(),
        }
    }

    /// L'appel système Linux x86-64 : le numéro est dans RAX, les
    /// arguments dans RDI, RSI, RDX (puis R10, R8, R9 — dont aucun appel
    /// géré ici n'a besoin), et la valeur de retour repart dans RAX.
    ///
    /// Deux appels seulement sont implémentés, ceux dont tout premier
    /// programme en assembleur a besoin pour faire quelque chose de
    /// visible : `write` et `exit`. namsdec n'est pas un noyau, et ne
    /// prétend pas l'être — un appel non géré s'arrête en le disant, ce
    /// qui apprend davantage qu'un code d'erreur silencieux.
    fn syscall(&mut self) -> Result<(), EmuError> {
        const SYS_READ: u64 = 0;
        const SYS_WRITE: u64 = 1;
        const SYS_EXIT: u64 = 60;
        /// `exit_group` termine tous les fils d'exécution. Sans threads,
        /// c'est un `exit` — et c'est celui que produisent la plupart des
        /// bibliothèques C.
        const SYS_EXIT_GROUP: u64 = 231;

        let number = self.cpu.regs.get64(Reg64::Rax);
        match number {
            SYS_READ => {
                let fd = self.cpu.regs.get64(Reg64::Rdi);
                let buf = self.cpu.regs.get64(Reg64::Rsi);
                let count = self.cpu.regs.get64(Reg64::Rdx);
                let read = self.read(fd, buf, count)?;
                self.cpu.regs.set64(Reg64::Rax, read as u64);
            }
            SYS_WRITE => {
                let fd = self.cpu.regs.get64(Reg64::Rdi);
                let buf = self.cpu.regs.get64(Reg64::Rsi);
                let count = self.cpu.regs.get64(Reg64::Rdx);
                let written = self.write(fd, buf, count)?;
                self.cpu.regs.set64(Reg64::Rax, written as u64);
            }
            SYS_EXIT | SYS_EXIT_GROUP => {
                // Le code de sortie est un octet : `exit(256)` rend 0, et
                // le tronquer ici évite de laisser croire le contraire.
                let code = self.cpu.regs.get64(Reg64::Rdi) as u8;
                self.exit_code = Some(code as i64);
                self.cpu.halted = true;
            }
            number => {
                return Err(EmuError::UnsupportedSyscall {
                    backend: Self::NAME,
                    number,
                });
            }
        }
        Ok(())
    }

    /// `write(fd, buf, count)`. Rend le nombre d'octets écrits, ou un code
    /// d'erreur négatif — la convention du noyau, qu'un programme en
    /// assembleur est censé savoir lire.
    fn write(&mut self, fd: u64, buf: u64, count: u64) -> Result<i64, EmuError> {
        /// Sortie standard et sortie d'erreur sont les seules ouvertes :
        /// il n'y a pas de système de fichiers derrière cet émulateur.
        /// Tout autre descripteur reçoit `-EBADF`, exactement comme le
        /// noyau répondrait à un descripteur jamais ouvert.
        const EBADF: i64 = -9;
        if fd != 1 && fd != 2 {
            return Ok(EBADF);
        }
        // Rien à lire, et surtout rien à vérifier en mémoire : un
        // `write` de zéro octet est valide et ne touche pas au tampon.
        if count == 0 {
            return Ok(0);
        }

        // La lecture passe par la mémoire simulée et ses bornes : un
        // pointeur qui sort de la fenêtre est une erreur d'exécution, pas
        // un moyen de lire ce qui traîne à côté.
        let bytes = self.cpu.mem.bytes_at(buf, count as usize)?;
        self.output.extend_from_slice(bytes);
        Ok(count as i64)
    }

    /// `read(fd, buf, count)`. Rend le nombre d'octets lus, 0 en fin
    /// d'entrée, ou un code d'erreur négatif.
    ///
    /// N'est jamais appelée quand il n'y a rien à lire et que l'entrée
    /// reste ouverte : `step` suspend l'exécution avant d'en arriver là
    /// (voir `blocks_on_input`). Ce qui suit n'a donc à traiter que les
    /// cas où `read` peut répondre tout de suite.
    fn read(&mut self, fd: u64, buf: u64, count: u64) -> Result<i64, EmuError> {
        /// L'entrée standard est le seul descripteur en lecture.
        const EBADF: i64 = -9;
        if fd != 0 {
            return Ok(EBADF);
        }
        if count == 0 {
            return Ok(0);
        }

        // Un `read` rend ce qu'il a sous la main, sans attendre d'avoir de
        // quoi remplir tout le tampon — c'est pour ça qu'un programme
        // sérieux boucle sur `read` au lieu de croire qu'un seul appel
        // suffit, et cette leçon-là mérite d'être apprise ici aussi.
        let take = (count as usize).min(self.input.len());
        if take == 0 {
            // Entrée close : fin de fichier.
            return Ok(0);
        }

        // L'entrée n'est retirée de la file qu'une fois écrite. Drainée
        // d'abord, elle était perdue quand le tampon de destination
        // tombait hors de la fenêtre mémoire : `step` rembobine RIP pour
        // rejouer le `syscall`, mais l'utilisateur devait retaper ce
        // qu'il avait déjà envoyé.
        let bytes: Vec<u8> = self.input.iter().take(take).copied().collect();
        self.record_memory(buf, bytes.len());
        self.cpu.mem.write_bytes(buf, &bytes)?;
        self.input.drain(..take);
        // L'entrée consommée est notée pour pouvoir être rendue : sans
        // ça, défaire un `read` laisserait le programme sans ce qu'il
        // vient de lire, et le rejeu se bloquerait sur une file vide.
        if let Some(undo) = self.history.back_mut() {
            undo.consumed_input.extend(bytes.iter().copied());
        }
        Ok(take as i64)
    }

    /// Cette instruction est-elle un `read` qui n'a rien à lire ?
    ///
    /// La question se pose *avant* d'exécuter quoi que ce soit : le
    /// programme doit être suspendu sans que RIP ni RAX ne bougent, pour
    /// que le même `syscall` se rejoue à l'identique quand l'entrée
    /// arrive. C'est ce qu'un vrai `read` bloquant fait du point de vue du
    /// programme, sans qu'on ait à bloquer le fil d'exécution de
    /// l'interface.
    fn blocks_on_input(&self, instr: &Instruction) -> bool {
        const SYS_READ: u64 = 0;
        instr.mnemonic() == Mnemonic::Syscall
            && self.cpu.regs.get64(Reg64::Rax) == SYS_READ
            && self.cpu.regs.get64(Reg64::Rdi) == 0
            && self.cpu.regs.get64(Reg64::Rdx) > 0
            && self.input.is_empty()
            && !self.input_closed
    }

    /// Décode l'instruction à RIP sans l'exécuter — partagé par `step()` et
    /// `step_over()` (qui doit inspecter l'instruction avant de décider
    /// comment avancer).
    fn decode_at_rip(&self) -> Result<Instruction, EmuError> {
        let rip = self.cpu.regs.rip;
        let window = fetch_window(&self.cpu.mem, rip)?.to_vec();
        namsdec_disasm::decode_one(&window, rip).ok_or(EmuError::InvalidInstruction { addr: rip })
    }

    /// Décode puis exécute l'instruction à RIP.
    pub fn step(&mut self) -> Result<StepResult, EmuError> {
        if self.cpu.halted {
            return Ok(StepResult::Halted);
        }

        let instr = self.decode_at_rip()?;

        if self.blocks_on_input(&instr) {
            return Ok(StepResult::NeedsInput);
        }

        // Par défaut RIP avance à l'instruction suivante ; les branchements
        // écrasent cette valeur pendant `execute`.
        let rip_before = self.cpu.regs.rip;
        self.begin_undo();
        self.cpu.regs.rip = instr.next_ip();
        if let Err(e) = self.execute(&instr) {
            // L'instruction n'a pas eu lieu : RIP doit rester dessus.
            // Sinon chaque tentative suivante avançait d'un cran — et
            // relancer le pas à pas après une erreur finissait par sortir
            // du code pour « exécuter » les données qui le suivent.
            //
            // Elle n'a pas non plus à figurer dans l'historique : on
            // défait tout de suite ce qu'elle a pu écrire avant d'échouer,
            // et sa page de journal disparaît avec.
            self.step_back();
            self.cpu.regs.rip = rip_before;
            return Err(e);
        }

        Ok(if self.cpu.halted {
            StepResult::Halted
        } else {
            StepResult::Continued
        })
    }

    /// Comme `step()`, sauf que si l'instruction courante est un `call`,
    /// toute la sous-routine appelée est exécutée d'un bloc plutôt que d'y
    /// entrer — le comportement "Step Over" standard des débogueurs.
    pub fn step_over(&mut self, max_steps: u32) -> Result<StepOverResult, EmuError> {
        if self.cpu.halted {
            return Ok(StepOverResult::Halted);
        }

        let instr = self.decode_at_rip()?;
        if instr.mnemonic() != Mnemonic::Call {
            return match self.step()? {
                StepResult::Halted => Ok(StepOverResult::Halted),
                StepResult::NeedsInput => Ok(StepOverResult::NeedsInput),
                StepResult::Continued => Ok(StepOverResult::Stepped),
            };
        }

        let return_addr = instr.next_ip();
        let start_rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);

        for _ in 0..max_steps {
            match self.step()? {
                StepResult::Halted => {
                    if let Some((addr, old, new)) = self.triggered_watchpoint() {
                        return Ok(StepOverResult::Watchpoint { addr, old, new });
                    }
                    return Ok(StepOverResult::Halted);
                }
                // Le `read` est au milieu de la sous-routine : on rend la
                // main, et `step_over` sera relancé quand l'entrée sera
                // là. La sous-routine reprendra donc là où elle en est,
                // et non depuis son début.
                StepResult::NeedsInput => return Ok(StepOverResult::NeedsInput),
                StepResult::Continued => {}
            }
            if let Some((addr, old, new)) = self.triggered_watchpoint() {
                return Ok(StepOverResult::Watchpoint { addr, old, new });
            }
            // La pile doit être revenue au moins à son niveau de départ pour
            // qu'on considère être "revenu" — protège contre un `ret`
            // orphelin qui atterrirait par coïncidence sur `return_addr` à
            // une profondeur de pile différente.
            if self.cpu.regs.rip == return_addr
                && self.cpu.regs.get64(namsdec_core::Reg64::Rsp) >= start_rsp
            {
                return Ok(StepOverResult::SteppedOverCall);
            }
            if self.breakpoints.contains(&self.cpu.regs.rip) {
                return Ok(StepOverResult::Breakpoint(self.cpu.regs.rip));
            }
        }
        Ok(StepOverResult::StepLimit)
    }

    /// Exécute jusqu'à `max_steps` instructions, jusqu'à `hlt`, un point
    /// d'arrêt (autre que celui de départ) ou la limite de pas.
    pub fn run(&mut self, max_steps: u32) -> Result<RunStop, EmuError> {
        for _ in 0..max_steps {
            if self.cpu.halted {
                return Ok(RunStop::Halted);
            }
            if self.step()? == StepResult::NeedsInput {
                return Ok(RunStop::NeedsInput);
            }
            // Avant même l'arrêt de la machine : si la dernière
            // instruction a touché une zone surveillée, c'est elle qu'on
            // vient voir, fût-elle la dernière du programme.
            if let Some((addr, old, new)) = self.triggered_watchpoint() {
                return Ok(RunStop::Watchpoint { addr, old, new });
            }
            if self.cpu.halted {
                return Ok(RunStop::Halted);
            }
            if self.breakpoints.contains(&self.cpu.regs.rip) {
                return Ok(RunStop::Breakpoint(self.cpu.regs.rip));
            }
        }
        Ok(RunStop::StepLimit)
    }

    fn execute(&mut self, instr: &Instruction) -> Result<(), EmuError> {
        use Mnemonic as M;
        match instr.mnemonic() {
            M::Mov => {
                let v = self.read_op(instr, 1)?;
                self.write_op(instr, 0, v)?;
            }
            M::Add => self.binary_arith(instr, add_with_flags, true)?,
            M::Sub => self.binary_arith(instr, sub_with_flags, true)?,
            M::Cmp => self.binary_arith(instr, sub_with_flags, false)?,
            M::And => self.binary_logic(instr, |a, b| a & b, true)?,
            M::Or => self.binary_logic(instr, |a, b| a | b, true)?,
            M::Xor => self.binary_logic(instr, |a, b| a ^ b, true)?,
            M::Test => self.binary_logic(instr, |a, b| a & b, false)?,
            M::Inc => self.inc_dec(instr, true)?,
            M::Dec => self.inc_dec(instr, false)?,
            M::Push => {
                let v = self.read_op(instr, 0)?;
                let new_rsp = self
                    .cpu
                    .regs
                    .get64(namsdec_core::Reg64::Rsp)
                    .wrapping_sub(8);
                self.record_memory(new_rsp, 8);
                self.cpu.mem.write_u64(new_rsp, v)?;
                self.cpu.regs.set64(namsdec_core::Reg64::Rsp, new_rsp);
            }
            M::Pop => {
                let rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);
                let v = self.cpu.mem.read_u64(rsp)?;
                // RSP avance *avant* l'écriture, comme sur un vrai
                // processeur : `pop rsp` charge RSP depuis la pile (la
                // valeur dépilée l'emporte), et `pop [rsp+8]` calcule son
                // adresse après l'incrément.
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rsp.wrapping_add(8));
                if let Err(e) = self.write_op(instr, 0, v) {
                    // Mais si l'écriture échoue, l'instruction n'a pas eu
                    // lieu : RSP retourne où il était, sans quoi chaque
                    // rejeu (que `step` provoque en rembobinant RIP)
                    // dépilerait un mot de plus.
                    self.cpu.regs.set64(namsdec_core::Reg64::Rsp, rsp);
                    return Err(e);
                }
            }
            M::Lea => {
                let addr = self.effective_address(instr, 1)?;
                self.write_op(instr, 0, addr)?;
            }

            // Extensions de taille : la source est plus étroite que la
            // destination, c'est tout l'intérêt de l'instruction — d'où
            // une largeur lue sur l'opérande source et non sur la
            // destination comme partout ailleurs.
            M::Movzx => {
                let v = self.read_op(instr, 1)?;
                self.write_op(instr, 0, v)?;
            }
            M::Movsx | M::Movsxd => {
                let v = self.read_op(instr, 1)?;
                let w = self.src_width(instr, 1)?;
                self.write_op(instr, 0, sign_extend(v, w))?;
            }

            M::Neg => {
                let w = self.dest_width(instr)?;
                let a = self.read_op(instr, 0)?;
                let (result, flags) = sub_with_flags(0, a, w);
                self.cpu.flags.apply_arith(flags);
                self.write_op(instr, 0, result)?;
            }
            M::Not => {
                // La seule opération logique qui ne touche aucun flag.
                let w = self.dest_width(instr)?;
                let a = self.read_op(instr, 0)?;
                self.write_op(instr, 0, !a & w.mask())?;
            }

            M::Adc => self.binary_carry(instr, true)?,
            M::Sbb => self.binary_carry(instr, false)?,

            M::Shl | M::Sal => self.shift(instr, ShiftKind::Left)?,
            M::Shr => self.shift(instr, ShiftKind::Right)?,
            M::Sar => self.shift(instr, ShiftKind::Arithmetic)?,

            M::Mul => self.multiply_accumulator(instr, false)?,
            M::Imul => self.imul(instr)?,
            M::Div => self.divide(instr, false)?,
            M::Idiv => self.divide(instr, true)?,

            M::Xchg => {
                let a = self.read_op(instr, 0)?;
                let b = self.read_op(instr, 1)?;
                self.write_op(instr, 0, b)?;
                self.write_op(instr, 1, a)?;
            }

            // `leave` défait ce que le prologue d'une fonction a monté :
            // RSP revient sur le cadre, puis RBP est dépilé.
            M::Leave => {
                let rbp = self.cpu.regs.get64(namsdec_core::Reg64::Rbp);
                let saved = self.cpu.mem.read_u64(rbp)?;
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rbp.wrapping_add(8));
                self.cpu.regs.set64(namsdec_core::Reg64::Rbp, saved);
            }

            // `loop` : RCX décrémenté *sans toucher aux flags*, puis saut
            // tant qu'il n'est pas nul.
            M::Loop => {
                let rcx = self
                    .cpu
                    .regs
                    .get64(namsdec_core::Reg64::Rcx)
                    .wrapping_sub(1);
                self.cpu.regs.set64(namsdec_core::Reg64::Rcx, rcx);
                if rcx != 0 {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }

            M::Sete
            | M::Setne
            | M::Setb
            | M::Setae
            | M::Setbe
            | M::Seta
            | M::Setl
            | M::Setge
            | M::Setle
            | M::Setg
            | M::Sets
            | M::Setns
            | M::Seto
            | M::Setno
            | M::Setp
            | M::Setnp => {
                let v = u64::from(self.condition_met(instr));
                self.write_op(instr, 0, v)?;
            }

            M::Cmove
            | M::Cmovne
            | M::Cmovb
            | M::Cmovae
            | M::Cmovbe
            | M::Cmova
            | M::Cmovl
            | M::Cmovge
            | M::Cmovle
            | M::Cmovg
            | M::Cmovs
            | M::Cmovns
            | M::Cmovo
            | M::Cmovno
            | M::Cmovp
            | M::Cmovnp => {
                if self.condition_met(instr) {
                    let v = self.read_op(instr, 1)?;
                    self.write_op(instr, 0, v)?;
                }
            }
            M::Nop => {}
            M::Jmp => {
                self.cpu.regs.rip = self.branch_target(instr)?;
            }
            M::Je
            | M::Jne
            | M::Jl
            | M::Jle
            | M::Jg
            | M::Jge
            | M::Jb
            | M::Jbe
            | M::Ja
            | M::Jae
            | M::Js
            | M::Jns
            | M::Jo
            | M::Jno
            | M::Jp
            | M::Jnp => {
                if self.condition_met(instr) {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }
            M::Call => {
                let target = self.branch_target(instr)?;
                // step() a déjà avancé RIP à next_ip avant d'appeler execute() :
                // c'est exactement l'adresse de retour à empiler.
                let ret_addr = self.cpu.regs.rip;
                let new_rsp = self
                    .cpu
                    .regs
                    .get64(namsdec_core::Reg64::Rsp)
                    .wrapping_sub(8);
                self.record_memory(new_rsp, 8);
                self.cpu.mem.write_u64(new_rsp, ret_addr)?;
                self.cpu.regs.set64(namsdec_core::Reg64::Rsp, new_rsp);
                self.cpu.regs.rip = target;
                self.frames.push(Frame {
                    // RIP a déjà avancé : l'adresse du `call` lui-même
                    // est celle d'où l'instruction a été lue.
                    call_site: instr.ip(),
                    entry: target,
                    return_addr: ret_addr,
                    rsp: new_rsp,
                });
                if let Some(undo) = self.history.back_mut() {
                    undo.frames = FrameChange::Pushed;
                }
            }
            M::Ret => {
                let rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);
                let ret_addr = self.cpu.mem.read_u64(rsp)?;
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rsp.wrapping_add(8));
                self.cpu.regs.rip = ret_addr;
                // Un `ret` sans `call` — du code qui saute dans une
                // sous-routine, ou une pile bricolée à la main — ne
                // dépile rien : la chaîne ne prétend pas savoir ce
                // qu'elle n'a pas vu.
                if let Some(frame) = self.frames.pop()
                    && let Some(undo) = self.history.back_mut()
                {
                    undo.frames = FrameChange::Popped(frame);
                }
            }
            M::Hlt => {
                self.cpu.halted = true;
            }
            M::Syscall => self.syscall()?,
            other => {
                return Err(EmuError::UnsupportedInstruction {
                    backend: Self::NAME,
                    mnemonic: format!("{other:?}"),
                });
            }
        }
        Ok(())
    }

    /// La condition d'un `jcc`, d'un `setcc` ou d'un `cmovcc`, lue sur
    /// l'instruction elle-même plutôt que déduite de son mnémonique :
    /// iced-x86 les ramène toutes à un même code, et les trois familles
    /// se partagent donc ce seul test.
    fn condition_met(&self, instr: &Instruction) -> bool {
        let f = &self.cpu.flags;
        let zf = f.contains(RFlags::ZF);
        let sf = f.contains(RFlags::SF);
        let cf = f.contains(RFlags::CF);
        let of = f.contains(RFlags::OF);
        let pf = f.contains(RFlags::PF);
        match instr.condition_code() {
            ConditionCode::e => zf,
            ConditionCode::ne => !zf,
            ConditionCode::b => cf,
            ConditionCode::ae => !cf,
            ConditionCode::be => cf || zf,
            ConditionCode::a => !cf && !zf,
            ConditionCode::l => sf != of,
            ConditionCode::ge => sf == of,
            ConditionCode::le => zf || (sf != of),
            ConditionCode::g => !zf && (sf == of),
            ConditionCode::s => sf,
            ConditionCode::ns => !sf,
            ConditionCode::o => of,
            ConditionCode::no => !of,
            ConditionCode::p => pf,
            ConditionCode::np => !pf,
            ConditionCode::None => unreachable!("instruction sans condition"),
        }
    }

    fn binary_arith(
        &mut self,
        instr: &Instruction,
        op: impl Fn(u64, u64, Width) -> (u64, RFlags),
        write_result: bool,
    ) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let a = self.read_op(instr, 0)?;
        let b = self.read_op(instr, 1)?;
        let (result, flags) = op(a, b, w);
        self.cpu.flags.apply_arith(flags);
        if write_result {
            self.write_op(instr, 0, result)?;
        }
        Ok(())
    }

    /// `adc`/`sbb` : l'addition et la soustraction qui tiennent compte de
    /// la retenue précédente — c'est par elles qu'on additionne des
    /// nombres plus larges qu'un registre.
    fn binary_carry(&mut self, instr: &Instruction, add: bool) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let a = self.read_op(instr, 0)?;
        let b = self.read_op(instr, 1)?;
        let carry = u64::from(self.cpu.flags.contains(RFlags::CF));

        // Deux opérations enchaînées plutôt qu'une : chacune peut
        // déborder, et `a + 0xff.. + 1` déborde deux fois.
        let (partial, first) = if add {
            add_with_flags(a, b, w)
        } else {
            sub_with_flags(a, b, w)
        };
        let (result, second) = if add {
            add_with_flags(partial, carry, w)
        } else {
            sub_with_flags(partial, carry, w)
        };
        let mut flags = second;
        for flag in [RFlags::CF, RFlags::OF, RFlags::AF] {
            if first.contains(flag) {
                flags.insert(flag);
            }
        }
        self.cpu.flags.apply_arith(flags);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    /// Décalages : `shl`/`sal`, `shr` et `sar`.
    ///
    /// Le compte est masqué comme sur le processeur (5 bits, 6 en 64
    /// bits), et un compte nul ne touche à aucun flag — c'est ce qui
    /// distingue `shl reg, cl` avec CL à zéro d'un décalage de 1. CF
    /// reçoit le dernier bit sorti ; OF n'a de sens que pour un décalage
    /// de 1, et AF reste indéfini (laissé à zéro).
    fn shift(&mut self, instr: &Instruction, kind: ShiftKind) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let bits = u64::from(w.bits());
        let a = self.read_op(instr, 0)? & w.mask();
        let count = self.read_op(instr, 1)? & if bits == 64 { 0x3f } else { 0x1f };
        if count == 0 {
            return Ok(());
        }

        let msb = 1u64 << (bits - 1);
        let (result, carry) = match kind {
            ShiftKind::Left => {
                let carry = count <= bits && (a >> (bits - count)) & 1 == 1;
                (a.wrapping_shl(count as u32) & w.mask(), carry)
            }
            ShiftKind::Right => {
                let carry = count <= bits && (a >> (count - 1)) & 1 == 1;
                (a >> count.min(63), carry)
            }
            ShiftKind::Arithmetic => {
                let signed = sign_extend(a, w) as i64;
                let carry = (signed >> (count - 1).min(63)) & 1 == 1;
                ((signed >> count.min(63)) as u64 & w.mask(), carry)
            }
        };

        let mut flags = logic_flags(result, w);
        flags.set(RFlags::CF, carry);
        if count == 1 {
            let overflow = match kind {
                ShiftKind::Left => ((result & msb) != 0) != carry,
                ShiftKind::Right => (a & msb) != 0,
                ShiftKind::Arithmetic => false,
            };
            flags.set(RFlags::OF, overflow);
        }
        self.cpu.flags.apply_arith(flags);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    /// `mul`, et la forme à un opérande d'`imul` : le produit est deux
    /// fois plus large que les opérandes, il déborde donc sur un second
    /// registre (AH, DX, EDX ou RDX selon la taille).
    fn multiply_accumulator(&mut self, instr: &Instruction, signed: bool) -> Result<(), EmuError> {
        let w = self.src_width(instr, 0)?;
        let src = self.read_op(instr, 0)?;
        let acc = self.accumulator(w);
        let mask = u128::from(w.mask());

        let full = if signed {
            ((sign_extend(acc, w) as i64 as i128) * (sign_extend(src, w) as i64 as i128)) as u128
        } else {
            u128::from(acc & w.mask()) * u128::from(src & w.mask())
        };
        let low = (full & mask) as u64;
        let high = ((full >> w.bits()) & mask) as u64;
        self.write_wide_result(w, low, high);

        // CF et OF disent la seule chose qui compte : le produit
        // tenait-il dans la largeur de départ ?
        let overflowed = if signed {
            sign_extend(low, w) != ((high << (w.bits() - 1)) | low)
                || (high != 0 && high != w.mask())
        } else {
            high != 0
        };
        self.cpu.flags.set(RFlags::CF, overflowed);
        self.cpu.flags.set(RFlags::OF, overflowed);
        Ok(())
    }

    /// `imul` a trois formes : un opérande (comme `mul`, en signé), deux
    /// (`dst *= src`) et trois (`dst = src * immédiat`). Les deux
    /// dernières gardent le produit dans la largeur de la destination.
    fn imul(&mut self, instr: &Instruction) -> Result<(), EmuError> {
        if instr.op_count() == 1 {
            return self.multiply_accumulator(instr, true);
        }
        let w = self.dest_width(instr)?;
        let (a, b) = if instr.op_count() == 2 {
            (self.read_op(instr, 0)?, self.read_op(instr, 1)?)
        } else {
            (self.read_op(instr, 1)?, self.read_op(instr, 2)?)
        };
        let full = (sign_extend(a, w) as i64 as i128) * (sign_extend(b, w) as i64 as i128);
        let result = (full as u64) & w.mask();
        // Le produit a-t-il été tronqué ? C'est tout ce que disent CF et
        // OF sur ces formes-là.
        let truncated = i128::from(sign_extend(result, w) as i64) != full;
        self.cpu.flags.set(RFlags::CF, truncated);
        self.cpu.flags.set(RFlags::OF, truncated);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    /// `div` et `idiv` : le dividende est deux fois plus large que le
    /// diviseur (AX, DX:AX, EDX:EAX, RDX:RAX), le quotient va dans
    /// l'accumulateur et le reste dans RDX (AH en 8 bits).
    ///
    /// Une division par zéro, comme un quotient trop grand pour le
    /// registre qui doit le recevoir, lève l'exception du processeur —
    /// ici une erreur qui arrête l'exécution en le disant, plutôt qu'un
    /// résultat inventé.
    fn divide(&mut self, instr: &Instruction, signed: bool) -> Result<(), EmuError> {
        use namsdec_core::Reg64::{Rax, Rdx};
        let w = self.src_width(instr, 0)?;
        let divisor = self.read_op(instr, 0)? & w.mask();
        if divisor == 0 {
            return Err(EmuError::DivideByZero {
                addr: self.cpu.regs.rip,
            });
        }

        let bits = w.bits();
        let low = self.accumulator(w) & w.mask();
        let high = match w {
            // En 8 bits le dividende est AX en entier : sa moitié haute
            // est AH, pas DX.
            Width::W8 => u64::from(self.cpu.regs.get16(Rax) >> 8),
            Width::W16 => u64::from(self.cpu.regs.get16(Rdx)),
            Width::W32 => u64::from(self.cpu.regs.get32(Rdx)),
            Width::W64 => self.cpu.regs.get64(Rdx),
        };
        let raw = (u128::from(high) << bits) | u128::from(low);

        let (quotient, remainder) = if signed {
            // Le dividende double largeur porte son signe sur son bit de
            // poids fort : il faut l'étendre avant de diviser.
            let shift = 128 - 2 * bits;
            let dividend = ((raw << shift) as i128) >> shift;
            let divisor = sign_extend(divisor, w) as i64 as i128;
            let quotient = dividend / divisor;
            if i128::from(sign_extend((quotient as u64) & w.mask(), w) as i64) != quotient {
                return Err(EmuError::DivideOverflow {
                    addr: self.cpu.regs.rip,
                });
            }
            (
                (quotient as u64) & w.mask(),
                ((dividend % divisor) as u64) & w.mask(),
            )
        } else {
            let quotient = raw / u128::from(divisor);
            if quotient > u128::from(w.mask()) {
                return Err(EmuError::DivideOverflow {
                    addr: self.cpu.regs.rip,
                });
            }
            (quotient as u64, (raw % u128::from(divisor)) as u64)
        };

        self.write_wide_result(w, quotient, remainder);
        Ok(())
    }

    /// Range une paire (bas, haut) là où le processeur la met : AX en 8
    /// bits, sinon l'accumulateur et RDX à la bonne largeur.
    fn write_wide_result(&mut self, w: Width, low: u64, high: u64) {
        use namsdec_core::Reg64::{Rax, Rdx};
        match w {
            Width::W8 => self
                .cpu
                .regs
                .set16(Rax, ((low & 0xff) | ((high & 0xff) << 8)) as u16),
            Width::W16 => {
                self.cpu.regs.set16(Rax, low as u16);
                self.cpu.regs.set16(Rdx, high as u16);
            }
            Width::W32 => {
                self.cpu.regs.set32(Rax, low as u32);
                self.cpu.regs.set32(Rdx, high as u32);
            }
            Width::W64 => {
                self.cpu.regs.set64(Rax, low);
                self.cpu.regs.set64(Rdx, high);
            }
        }
    }

    /// AL, AX, EAX ou RAX selon la largeur.
    fn accumulator(&self, w: Width) -> u64 {
        use namsdec_core::Reg64::Rax;
        match w {
            Width::W8 => u64::from(self.cpu.regs.get8l(Rax)),
            Width::W16 => u64::from(self.cpu.regs.get16(Rax)),
            Width::W32 => u64::from(self.cpu.regs.get32(Rax)),
            Width::W64 => self.cpu.regs.get64(Rax),
        }
    }

    /// La largeur d'un opérande donné — celle de la *source* quand elle
    /// diffère de la destination (`movzx`, `movsx`, `mul`, `div`).
    fn src_width(&self, instr: &Instruction, op_index: u32) -> Result<Width, EmuError> {
        match instr.op_kind(op_index) {
            OpKind::Register => Ok(map_register(instr.op_register(op_index))?.1),
            OpKind::Memory => self.mem_operand_width(instr),
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Read,
                kind: format!("{other:?}"),
            }),
        }
    }

    fn binary_logic(
        &mut self,
        instr: &Instruction,
        op: impl Fn(u64, u64) -> u64,
        write_result: bool,
    ) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let a = self.read_op(instr, 0)?;
        let b = self.read_op(instr, 1)?;
        let result = op(a, b);
        self.cpu.flags.apply_arith(logic_flags(result, w));
        if write_result {
            self.write_op(instr, 0, result)?;
        }
        Ok(())
    }

    /// Adresse cible d'un `jmp`/`call` : `near_branch_target()` pour les
    /// formes relatives (rel8/rel32, l'immense majorité en pratique), sinon
    /// l'opérande générique (registre ou mémoire indirects). Sans ce
    /// dispatch, `near_branch_target()` sur une forme indirecte renvoie
    /// silencieusement 0 plutôt que d'échouer — un bug latent qu'avait
    /// `Jmp` avant l'ajout de `Call`.
    fn branch_target(&self, instr: &Instruction) -> Result<u64, EmuError> {
        match instr.op_kind(0) {
            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                Ok(instr.near_branch_target())
            }
            _ => self.read_op(instr, 0),
        }
    }

    /// INC/DEC affectent OF/SF/ZF/AF/PF comme add/sub de 1, mais
    /// préservent CF (comportement x86 réel).
    fn inc_dec(&mut self, instr: &Instruction, increment: bool) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let a = self.read_op(instr, 0)?;
        let (result, flags) = if increment {
            add_with_flags(a, 1, w)
        } else {
            sub_with_flags(a, 1, w)
        };
        let cf = self.cpu.flags.contains(RFlags::CF);
        self.cpu.flags.apply_arith(flags);
        self.cpu.flags.set(RFlags::CF, cf);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    fn dest_width(&self, instr: &Instruction) -> Result<Width, EmuError> {
        match instr.op_kind(0) {
            OpKind::Register => Ok(map_register(instr.op_register(0))?.1),
            OpKind::Memory => self.mem_operand_width(instr),
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Destination,
                kind: format!("{other:?}"),
            }),
        }
    }

    fn mem_operand_width(&self, instr: &Instruction) -> Result<Width, EmuError> {
        width_from_bytes(instr.memory_size().size()).ok_or_else(|| {
            EmuError::UnsupportedOperandSize {
                backend: Self::NAME,
                bytes: instr.memory_size().size(),
            }
        })
    }

    fn read_op(&self, instr: &Instruction, op_index: u32) -> Result<u64, EmuError> {
        match instr.op_kind(op_index) {
            OpKind::Register => self.get_reg(instr.op_register(op_index)),
            OpKind::Memory => {
                let addr = self.effective_address(instr, op_index)?;
                let w = self.mem_operand_width(instr)?;
                self.read_mem(addr, w)
            }
            OpKind::Immediate8
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate64
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate32to64 => Ok(instr.immediate(op_index)),
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Read,
                kind: format!("{other:?}"),
            }),
        }
    }

    fn write_op(&mut self, instr: &Instruction, op_index: u32, value: u64) -> Result<(), EmuError> {
        match instr.op_kind(op_index) {
            OpKind::Register => self.set_reg(instr.op_register(op_index), value),
            OpKind::Memory => {
                let addr = self.effective_address(instr, op_index)?;
                let w = self.mem_operand_width(instr)?;
                self.write_mem(addr, w, value)
            }
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Write,
                kind: format!("{other:?}"),
            }),
        }
    }

    /// Ouvre une page de journal pour le pas qui commence.
    fn begin_undo(&mut self) {
        if self.history.len() >= HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.history.push_back(Undo {
            regs: self.cpu.regs.clone(),
            flags: self.cpu.flags,
            halted: self.cpu.halted,
            exit_code: self.exit_code,
            output_len: self.output.len(),
            consumed_input: Vec::new(),
            input_closed: self.input_closed,
            memory: Vec::new(),
            frames: FrameChange::None,
        });
    }

    /// Note les octets sur le point d'être écrasés. Lus *avant*
    /// l'écriture, évidemment — c'est toute l'idée.
    fn record_memory(&mut self, addr: u64, len: usize) {
        if self.history.is_empty() {
            return;
        }
        let Ok(old) = self.cpu.mem.bytes_at(addr, len) else {
            return;
        };
        let old = old.to_vec();
        if let Some(undo) = self.history.back_mut() {
            undo.memory.push((addr, old));
        }
    }

    /// Défait le dernier pas : les registres, les flags, l'arrêt, la
    /// sortie écrite, l'entrée consommée et les octets écrasés reviennent
    /// à ce qu'ils étaient.
    ///
    /// C'est ce qu'un émulateur peut offrir et qu'un débogueur ordinaire
    /// peine à rendre : on rejoue une instruction autant de fois qu'il le
    /// faut pour la comprendre, au lieu de tout relancer depuis le début.
    pub fn step_back(&mut self) -> bool {
        let Some(undo) = self.history.pop_back() else {
            return false;
        };
        // Dans l'ordre inverse des écritures : deux écritures au même
        // endroit doivent se défaire en commençant par la dernière.
        for (addr, bytes) in undo.memory.iter().rev() {
            let _ = self.cpu.mem.write_bytes(*addr, bytes);
        }
        self.cpu.regs = undo.regs;
        self.cpu.flags = undo.flags;
        self.cpu.halted = undo.halted;
        self.exit_code = undo.exit_code;
        self.output.truncate(undo.output_len);
        for byte in undo.consumed_input.iter().rev() {
            self.input.push_front(*byte);
        }
        self.input_closed = undo.input_closed;
        match undo.frames {
            FrameChange::None => {}
            FrameChange::Pushed => {
                self.frames.pop();
            }
            FrameChange::Popped(frame) => self.frames.push(frame),
        }
        // Revenir en arrière ne déclenche pas les points de
        // surveillance : ils regardent ce qui vient, pas ce qu'on défait.
        self.rearm_watchpoints();
        true
    }

    /// Les appels en cours, du plus ancien au plus récent.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Remplace les zones surveillées, et les arme sur l'état courant :
    /// poser un point de surveillance, c'est dire « préviens-moi si ça
    /// change **à partir de maintenant** ».
    pub fn set_watchpoints(&mut self, points: &[(u64, usize)]) {
        self.watchpoints = points
            .iter()
            .map(|&(addr, len)| Watchpoint {
                addr,
                len: len.clamp(1, 8),
                previous: self.peek(addr, len.clamp(1, 8)),
            })
            .collect();
    }

    pub fn watchpoints(&self) -> &[Watchpoint] {
        &self.watchpoints
    }

    /// Réarme les zones surveillées sur l'état courant, sans rien
    /// signaler. Après un retour en arrière, c'est d'ici qu'on repart.
    fn rearm_watchpoints(&mut self) {
        for point in &mut self.watchpoints {
            let (addr, len) = (point.addr, point.len);
            // Emprunt : la lecture passe par `peek`, qui ne touche à rien.
            let value = read_bytes(&self.cpu.mem, addr, len);
            point.previous = value;
        }
    }

    /// La première zone surveillée dont la valeur a changé.
    fn triggered_watchpoint(&mut self) -> Option<(u64, u64, u64)> {
        for index in 0..self.watchpoints.len() {
            let (addr, len, old) = {
                let point = &self.watchpoints[index];
                (point.addr, point.len, point.previous)
            };
            let new = read_bytes(&self.cpu.mem, addr, len);
            if new != old {
                self.watchpoints[index].previous = new;
                return Some((addr, old, new));
            }
        }
        None
    }

    fn peek(&self, addr: u64, len: usize) -> u64 {
        read_bytes(&self.cpu.mem, addr, len)
    }

    /// Y a-t-il quelque chose à défaire ?
    pub fn can_step_back(&self) -> bool {
        !self.history.is_empty()
    }

    fn get_reg(&self, reg: Register) -> Result<u64, EmuError> {
        let (r, w) = map_register(reg)?;
        Ok(match w {
            Width::W64 => self.cpu.regs.get64(r),
            Width::W32 => self.cpu.regs.get32(r) as u64,
            Width::W16 => self.cpu.regs.get16(r) as u64,
            Width::W8 => self.cpu.regs.get8l(r) as u64,
        })
    }

    fn set_reg(&mut self, reg: Register, value: u64) -> Result<(), EmuError> {
        let (r, w) = map_register(reg)?;
        match w {
            Width::W64 => self.cpu.regs.set64(r, value),
            Width::W32 => self.cpu.regs.set32(r, value as u32),
            Width::W16 => self.cpu.regs.set16(r, value as u16),
            Width::W8 => self.cpu.regs.set8l(r, value as u8),
        }
        Ok(())
    }

    fn effective_address(&self, instr: &Instruction, op_index: u32) -> Result<u64, EmuError> {
        let next_ip = instr.next_ip();
        instr
            .virtual_address(op_index, 0, |register, _idx, _size| match register {
                Register::None => Some(0),
                Register::RIP | Register::EIP => Some(next_ip),
                // iced réclame aussi le registre de segment : une adresse
                // virtuelle, c'est la base du segment plus le décalage.
                // En 64 bits, CS/DS/ES/SS ont une base nulle — c'est le
                // modèle plat, et c'est ce que Linux donne à un
                // programme. Sans ce cas, *aucun* opérande mémoire ne se
                // calculait : `mov [case], rax` échouait avant même de
                // lire la mémoire.
                Register::CS | Register::DS | Register::ES | Register::SS => Some(0),
                // FS et GS, eux, ont une base que le noyau installe (TLS,
                // canari de pile). namsdec ne la simule pas : mieux vaut
                // refuser l'instruction que calculer une adresse fausse.
                Register::FS | Register::GS => None,
                other => map_register(other)
                    .ok()
                    .map(|(r, _)| self.cpu.regs.get64(r)),
            })
            .ok_or(EmuError::InvalidInstruction { addr: instr.ip() })
    }

    fn read_mem(&self, addr: u64, w: Width) -> Result<u64, EmuError> {
        Ok(match w {
            Width::W8 => self.cpu.mem.read_u8(addr)? as u64,
            Width::W16 => self.cpu.mem.read_u16(addr)? as u64,
            Width::W32 => self.cpu.mem.read_u32(addr)? as u64,
            Width::W64 => self.cpu.mem.read_u64(addr)?,
        })
    }

    fn write_mem(&mut self, addr: u64, w: Width, value: u64) -> Result<(), EmuError> {
        self.record_memory(addr, (w.bits() / 8) as usize);
        match w {
            Width::W8 => self.cpu.mem.write_u8(addr, value as u8)?,
            Width::W16 => self.cpu.mem.write_u16(addr, value as u16)?,
            Width::W32 => self.cpu.mem.write_u32(addr, value as u32)?,
            Width::W64 => self.cpu.mem.write_u64(addr, value)?,
        }
        Ok(())
    }
}

fn width_from_bytes(bytes: usize) -> Option<Width> {
    match bytes {
        1 => Some(Width::W8),
        2 => Some(Width::W16),
        4 => Some(Width::W32),
        8 => Some(Width::W64),
        _ => None,
    }
}

fn fetch_window(mem: &Memory, addr: u64) -> Result<&[u8], EmuError> {
    let range = mem.range();
    if addr < range.start || addr >= range.end {
        return Err(EmuError::InvalidInstruction { addr });
    }
    let avail = (range.end - addr).min(16) as usize;
    mem.bytes_at(addr, avail)
        .map_err(|_| EmuError::InvalidInstruction { addr })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une instruction que l'interpréteur ne sait pas exécuter laisse RIP
    /// où il est. Sans ça, réessayer avançait d'un cran à chaque fois et
    /// l'exécution finissait par décoder les données comme du code.
    #[test]
    fn a_failed_step_leaves_rip_on_the_offending_instruction() {
        let mut mem = namsdec_core::Memory::new(0x400000, 0x1000);
        // `cpuid` (0F A2) : parfaitement décodable, mais pas exécutée par
        // cet interpréteur.
        mem.load(0x400000, &[0x0f, 0xa2]).unwrap();
        let mut cpu = namsdec_core::Cpu::new(mem);
        cpu.regs.rip = 0x400000;
        let mut emu = Emulator::new(cpu);

        for attempt in 1..=3 {
            let err = emu.step().expect_err("cpuid n'est pas supporté");
            assert!(
                matches!(err, EmuError::UnsupportedInstruction { .. }),
                "obtenu {err:?}"
            );
            assert_eq!(
                emu.cpu.regs.rip, 0x400000,
                "RIP a bougé à la tentative {attempt}"
            );
        }
    }
    use namsdec_core::Reg64;

    const BASE: u64 = 0x400000;

    /// Assemble `asm` avec `namsdec-asm`, le charge en mémoire et exécute
    /// jusqu'à l'arrêt (halt, point d'arrêt ou limite de pas). Test
    /// d'intégration "golden" : il passe par le vrai `nasm` externe.
    fn run_program(asm: &str, breakpoints: &[u64]) -> Emulator {
        let mut emu = load_only(asm, breakpoints);
        emu.run(10_000).expect("exécution");
        emu
    }

    /// Le programme que tout le monde écrit en premier : il parle, puis
    /// il sort. C'est exactement ce que `write` et `exit` servent à faire.
    #[test]
    fn a_program_can_write_to_stdout_and_exit_with_a_code() {
        let emu = run_program(
            "\
section .data
    message db \"salut\", 10

section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, message
    mov rdx, 6
    syscall

    mov rax, 60
    mov rdi, 3
    syscall
",
            &[],
        );

        assert_eq!(emu.output(), b"salut\n");
        assert_eq!(emu.exit_code(), Some(3));
        assert!(emu.cpu.halted, "`exit` arrête la machine");
    }

    /// `write` rend le nombre d'octets écrits, comme le noyau — un
    /// programme peut donc le vérifier, et le `hlt` juste après laisse la
    /// valeur observable dans RAX.
    #[test]
    fn write_returns_the_number_of_bytes_written() {
        let emu = run_program(
            "\
section .data
    message db \"salut\", 10

section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, message
    mov rdx, 6
    syscall
    hlt
",
            &[],
        );

        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
        assert_eq!(emu.output(), b"salut\n");
    }

    /// Un programme qui lit son entrée : tant qu'on ne lui a rien donné,
    /// il attend — sans avancer d'un pouce, et sans bloquer l'interface.
    /// Dès que l'entrée arrive, le même `syscall` repart et la lit.
    #[test]
    fn read_waits_for_input_then_consumes_it() {
        const PROGRAM: &str = "\
section .bss
    tampon resb 32

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 32
    syscall
    hlt
";
        let mut emu = load_only(PROGRAM, &[]);

        // Le programme court jusqu'au `read`, puis s'arrête là.
        assert_eq!(emu.run(100).unwrap(), RunStop::NeedsInput);
        let waiting_at = emu.cpu.regs.rip;
        assert!(!emu.cpu.halted, "attendre n'est pas finir");

        // Insister ne sert à rien et ne casse rien : RIP ne bouge pas.
        assert_eq!(emu.step().unwrap(), StepResult::NeedsInput);
        assert_eq!(emu.cpu.regs.rip, waiting_at);

        emu.provide_input(b"bonjour\n");
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);

        // `read` a rendu le nombre d'octets lus...
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 8);
        // ...et les a bien déposés dans le tampon du programme.
        let buf = emu.cpu.regs.get64(Reg64::Rsi);
        assert_eq!(emu.cpu.mem.bytes_at(buf, 8).unwrap(), b"bonjour\n");
        assert_eq!(emu.pending_input(), 0, "tout a été consommé");
    }

    /// Une entrée close rend une fin de fichier (0) au lieu d'attendre —
    /// l'équivalent d'un Ctrl+D. Sans ça, un programme qui lit jusqu'à la
    /// fin de son entrée attendrait pour toujours.
    #[test]
    fn a_closed_input_reads_as_end_of_file() {
        let mut emu = load_only(
            "\
section .bss
    tampon resb 8

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 8
    syscall
    hlt
",
            &[],
        );

        emu.close_input();
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rax),
            0,
            "attendu une fin de fichier"
        );
    }

    /// `read` rend ce qu'il a sous la main plutôt que d'attendre de quoi
    /// remplir tout le tampon — c'est pourquoi un programme sérieux boucle
    /// sur `read`.
    #[test]
    fn read_returns_what_is_available_without_filling_the_buffer() {
        let mut emu = load_only(
            "\
section .bss
    tampon resb 64

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 64
    syscall
    hlt
",
            &[],
        );

        emu.provide_input(b"court");
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 5);
    }

    /// Lire depuis autre chose que l'entrée standard n'a pas de sens ici :
    /// même réponse que le noyau pour un descripteur jamais ouvert.
    #[test]
    fn reading_from_an_unknown_file_descriptor_returns_ebadf() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 7
    mov rsi, 0
    mov rdx, 8
    syscall
    hlt
",
            &[],
        );

        assert_eq!(emu.cpu.regs.get64(Reg64::Rax) as i64, -9);
    }

    /// `hlt` arrête la machine sans que le programme ait rendu quoi que ce
    /// soit : il n'y a pas de code de sortie à inventer.
    #[test]
    fn halting_is_not_exiting() {
        let emu = run_program("section .text\n    mov rax, 1\n    hlt\n", &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.exit_code(), None);
    }

    /// Il n'y a pas de système de fichiers derrière cet émulateur : tout
    /// descripteur autre que 1 et 2 reçoit `-EBADF`, la réponse qu'aurait
    /// donnée le noyau. Un programme en assembleur est censé savoir lire
    /// ce retour négatif — c'est la convention, pas un détail d'émulation.
    #[test]
    fn writing_to_an_unknown_file_descriptor_returns_ebadf() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 7
    mov rsi, 0
    mov rdx, 1
    syscall
    hlt
",
            &[],
        );

        assert_eq!(emu.cpu.regs.get64(Reg64::Rax) as i64, -9, "attendu -EBADF");
        assert!(emu.output().is_empty(), "rien ne doit avoir été écrit");
    }

    /// Un appel système que namsdec ne simule pas s'arrête en le disant.
    /// Mieux vaut cet aveu qu'un code d'erreur inventé, qui laisserait
    /// croire que l'appel a eu lieu.
    #[test]
    fn an_unimplemented_syscall_says_so() {
        // 39 = `getpid`.
        let mut emu = load_only(
            "section .text\n    global _start\n_start:\n    mov rax, 39\n    syscall\n    hlt\n",
            &[],
        );
        let entry = emu.cpu.regs.rip;

        let err = emu.run(100).expect_err("getpid n'est pas simulé");
        match err {
            EmuError::UnsupportedSyscall { number, .. } => assert_eq!(number, 39),
            other => panic!("attendu UnsupportedSyscall, obtenu {other:?}"),
        }
        assert!(
            emu.cpu.regs.rip > entry,
            "les instructions avant le syscall se sont bien exécutées"
        );
        assert!(!emu.cpu.halted, "un appel refusé n'arrête pas la machine");
    }

    /// Un pointeur qui sort de la fenêtre mémoire simulée est une erreur,
    /// pas une occasion de lire ce qui traîne à côté.
    #[test]
    fn writing_from_an_address_outside_memory_fails() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, 0xdeadbeef
    mov rdx, 4
    syscall
    hlt
",
            &[],
        );

        let err = emu
            .run(100)
            .expect_err("l'adresse est hors de la mémoire simulée");
        assert!(matches!(err, EmuError::Mem(_)), "obtenu {err:?}");
        assert!(emu.output().is_empty());
    }

    /// Un `write` de zéro octet est valide et ne regarde même pas le
    /// pointeur — y compris quand celui-ci ne pointe nulle part.
    #[test]
    fn writing_zero_bytes_is_allowed_and_writes_nothing() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, 0xdeadbeef
    mov rdx, 0
    syscall
    hlt
",
            &[],
        );

        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
        assert!(emu.output().is_empty());
    }

    /// Charge sans exécuter — pour les programmes dont on veut observer
    /// l'échec plutôt que le résultat.
    /// Un point de surveillance arrête l'exécution quand la mémoire
    /// change sous lui — sans qu'on ait eu à deviner quelle instruction
    /// le ferait, ce qui est tout l'intérêt.
    #[test]
    fn a_watchpoint_stops_the_run_when_the_value_changes() {
        let mut emu = load_only(
            "\
section .data
    compteur dq 0

section .text
    global _start
_start:
    mov rax, 1
    mov rbx, 2
    mov rcx, 3
    mov [compteur], rcx     ; c'est ici que ça change
    mov rdx, 4
    hlt
",
            &[],
        );
        let compteur = namsdec_asm::assemble(
            "section .data\n    compteur dq 0\nsection .text\n    global _start\n_start:\n    hlt\n",
            BASE,
        )
        .unwrap()
        .sections
        .iter()
        .find(|s| s.name == ".data")
        .expect("une section .data")
        .addr;

        emu.set_watchpoints(&[(compteur, 8)]);
        assert_eq!(emu.watchpoints().len(), 1);
        assert_eq!(
            emu.watchpoints()[0].value(),
            0,
            "armé sur la valeur du moment"
        );

        let stop = emu.run(100).expect("exécution");
        match stop {
            RunStop::Watchpoint { addr, old, new } => {
                assert_eq!(addr, compteur);
                assert_eq!((old, new), (0, 3), "la valeur d'avant et celle d'après");
            }
            other => panic!("attendu un arrêt sur surveillance, obtenu {other:?}"),
        }
        // Arrêté *après* l'instruction fautive, pas à la fin du programme.
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rdx),
            0,
            "`mov rdx, 4` n'a pas eu lieu"
        );
        assert!(!emu.cpu.halted);

        // Relancé, il va jusqu'au bout : la zone ne change plus.
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
    }

    /// Une écriture qui ne change rien n'arrête rien : `mov [x], 0` sur
    /// une case déjà nulle n'apprend rien à personne.
    #[test]
    fn writing_the_same_value_does_not_trigger_a_watchpoint() {
        let mut emu = load_only(
            "\
section .bss
    case resq 1

section .text
    global _start
_start:
    xor rax, rax
    mov [case], rax
    hlt
",
            &[],
        );
        let case = BASE + 0x1000; // .bss, la seule donnée
        emu.set_watchpoints(&[(case, 8)]);
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted, "rien n'a changé");
    }

    /// Le retour en arrière réarme les zones surveillées : il défait, il
    /// ne déclenche pas.
    #[test]
    fn stepping_back_rearms_the_watchpoints_instead_of_firing_them() {
        let mut emu = load_only(
            "\
section .bss
    case resq 1

section .text
    global _start
_start:
    mov rax, 5
    mov [case], rax
    hlt
",
            &[],
        );
        let case = BASE + 0x1000;
        emu.set_watchpoints(&[(case, 8)]);
        assert!(matches!(
            emu.run(100).unwrap(),
            RunStop::Watchpoint { old: 0, new: 5, .. }
        ));

        // On revient avant l'écriture : la zone est réarmée sur zéro, et
        // rien ne s'est déclenché au passage.
        assert!(emu.step_back());
        assert_eq!(emu.watchpoints()[0].value(), 0);

        // Et rejouer le même pas la redéclenche.
        assert!(matches!(
            emu.run(100).unwrap(),
            RunStop::Watchpoint { old: 0, new: 5, .. }
        ));
    }

    /// La chaîne des appels se lit directement : l'émulateur voit passer
    /// les `call` et les `ret`, il n'a pas à deviner les cadres en
    /// suivant RBP — ce que le code d'un débutant ne monte pas.
    #[test]
    fn the_call_chain_follows_calls_and_returns() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    call externe
    hlt

externe:
    call interne
    ret

interne:
    mov rax, 1
    ret
",
            &[],
        );
        assert!(emu.frames().is_empty(), "au départ, personne n'est appelé");

        emu.step().unwrap(); // call externe
        assert_eq!(emu.frames().len(), 1);
        assert_eq!(
            emu.frames()[0].call_site,
            BASE,
            "le `call` est la première instruction"
        );

        emu.step().unwrap(); // call interne
        assert_eq!(emu.frames().len(), 2, "deux appels imbriqués");
        let inner = emu.frames()[1];
        assert_eq!(inner.entry, emu.cpu.regs.rip, "on est entré dans `interne`");
        assert!(inner.call_site > emu.frames()[0].call_site);

        emu.step().unwrap(); // mov rax, 1
        emu.step().unwrap(); // ret
        assert_eq!(emu.frames().len(), 1, "le retour dépile son cadre");
        emu.step().unwrap(); // ret
        assert!(emu.frames().is_empty(), "de retour au programme principal");

        // Un `ret` de trop ne dépile rien qui n'existe pas.
        emu.cpu.regs.rip = inner.return_addr;
        assert!(emu.frames().is_empty());
    }

    /// Et le pas arrière remet la chaîne comme elle était : défaire un
    /// `call` retire son cadre, défaire un `ret` le remet.
    #[test]
    fn stepping_back_restores_the_call_chain() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    call externe
    hlt

externe:
    mov rax, 1
    ret
",
            &[],
        );
        emu.step().unwrap(); // call
        let called = emu.frames().to_vec();
        assert_eq!(called.len(), 1);

        emu.step().unwrap(); // mov
        emu.step().unwrap(); // ret
        assert!(emu.frames().is_empty());

        emu.step_back(); // défait le ret
        assert_eq!(emu.frames(), called.as_slice(), "le cadre est revenu");

        emu.step_back(); // défait le mov
        emu.step_back(); // défait le call
        assert!(emu.frames().is_empty(), "et il repart avec son `call`");
    }

    /// Lire et écrire *en mémoire* par un opérande — le pain quotidien
    /// dès qu'on manipule un tableau.
    ///
    /// Aucune de ces formes ne fonctionnait : iced réclame le registre de
    /// segment pour calculer une adresse virtuelle, et l'interpréteur ne
    /// savait pas quoi en faire. Il refusait donc l'instruction avant
    /// même de toucher à la mémoire — et rien ne le disait, faute d'un
    /// test qui déréférence quoi que ce soit.
    #[test]
    fn memory_operands_are_addressable_in_every_usual_form() {
        let emu = run_program(
            "\
section .data
    table dq 10, 20, 30, 40

section .bss
    case resq 1

section .text
    global _start
_start:
    mov rax, [table]            ; adresse absolue
    mov rsi, table
    mov rbx, [rsi + 8]          ; base + décalage
    mov rcx, 2
    mov rdx, [rsi + rcx*8]      ; base + index mis à l'échelle
    mov r8, 99
    mov [case], r8              ; écriture absolue
    mov r9, [case]              ; relecture
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 10, "[table]");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 20, "[rsi + 8]");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 30, "[rsi + rcx*8]");
        assert_eq!(emu.cpu.regs.get64(Reg64::R9), 99, "ce qu'on vient d'écrire");
    }

    /// Le pas arrière défait ce que le dernier pas a fait : registres,
    /// flags, et la mémoire écrite au passage.
    #[test]
    fn stepping_back_undoes_registers_flags_and_memory() {
        let mut emu = load_only(
            "\
section .bss
    case resq 1

section .text
    global _start
_start:
    mov rax, 7
    mov [case], rax
    cmp rax, 7          ; ZF = 1
    hlt
",
            &[],
        );
        for _ in 0..3 {
            emu.step().unwrap();
        }
        let case = emu.cpu.regs.rip;
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 7);
        assert!(emu.cpu.flags.contains(RFlags::ZF), "le cmp a levé ZF");

        // On défait le `cmp` : ZF retombe, RIP recule.
        assert!(emu.step_back());
        assert!(
            !emu.cpu.flags.contains(RFlags::ZF),
            "ZF est revenu comme avant"
        );
        assert!(emu.cpu.regs.rip < case);

        // Puis l'écriture en mémoire, et enfin le `mov rax, 7` : RAX
        // redevient nul.
        assert!(emu.step_back());
        assert!(emu.step_back());
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0, "RAX est revenu à zéro");

        // Plus rien à défaire au départ du programme.
        assert!(!emu.can_step_back());
        assert!(!emu.step_back());
    }

    /// Ce qu'un `mov [adresse], valeur` a écrit est bien remis en place,
    /// et pas seulement les registres.
    #[test]
    fn stepping_back_restores_the_bytes_that_were_overwritten() {
        let mut emu = load_only(
            "\
section .data
    case dq 0x1122334455667788

section .text
    global _start
_start:
    mov rax, 0
    mov [case], rax
    hlt
",
            &[],
        );
        let addr = {
            // La case est la seule donnée : elle est au début de .data.
            let out = namsdec_asm::assemble(
                "section .data\n    case dq 0x1122334455667788\nsection .text\n    global _start\n_start:\n    hlt\n",
                BASE,
            )
            .unwrap();
            out.sections
                .iter()
                .find(|s| s.name == ".data")
                .expect("une section .data")
                .addr
        };

        emu.step().unwrap(); // mov rax, 0
        emu.step().unwrap(); // mov [case], rax
        assert_eq!(
            emu.cpu.mem.read_u64(addr).unwrap(),
            0,
            "la case est écrasée"
        );

        assert!(emu.step_back());
        assert_eq!(
            emu.cpu.mem.read_u64(addr).unwrap(),
            0x1122334455667788,
            "les octets d'origine sont revenus"
        );
    }

    /// Défaire un `write` reprend ce que le programme avait affiché, et
    /// défaire un `read` lui rend ce qu'il avait consommé — sans quoi le
    /// rejeu se bloquerait sur une entrée vide.
    #[test]
    fn stepping_back_rewinds_the_console_too() {
        let mut emu = load_only(
            "\
section .bss
    tampon resb 8

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 3
    syscall             ; read
    mov rax, 1
    mov rdi, 1
    mov rsi, tampon
    mov rdx, 3
    syscall             ; write
    hlt
",
            &[],
        );
        emu.provide_input(b"oui");
        emu.run(100).unwrap();
        assert_eq!(emu.output(), b"oui");
        assert_eq!(emu.pending_input(), 0);

        // Défaire le `write` : la sortie se vide.
        while emu.output() == b"oui" {
            assert!(emu.step_back());
        }
        assert!(emu.output().is_empty(), "la sortie est reprise");

        // Continuer jusqu'à défaire le `read` : l'entrée revient.
        while emu.pending_input() == 0 && emu.can_step_back() {
            emu.step_back();
        }
        assert_eq!(emu.pending_input(), 3, "les octets lus sont rendus");
    }

    /// Le lot d'instructions qui manquait pour aller au-delà des
    /// premiers exercices : multiplication, division, décalages,
    /// extensions de taille, `setcc`. Chacune est vérifiée sur son
    /// résultat *et* sur les flags qu'un cours fait lire juste après.
    #[test]
    fn multiplication_and_division_land_in_the_right_registers() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 7
    mov rbx, 6
    mul rbx             ; RDX:RAX = 42
    mov rcx, rax

    mov rax, 100
    xor rdx, rdx        ; la moitié haute du dividende doit être nette
    mov rbx, 7
    div rbx             ; RAX = 14, RDX = 2
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 42, "7 * 6");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 14, "100 / 7");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 2, "et son reste");
    }

    /// `imul` à deux opérandes garde le produit dans la destination, et
    /// signale par CF/OF qu'il a dû le tronquer.
    #[test]
    fn imul_reports_a_truncated_product() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov eax, -3
    imul eax, 5         ; -15, tient dans 32 bits
    mov ebx, eax

    mov eax, 0x40000000
    imul eax, 4         ; 2^32 : ne tient pas
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get32(Reg64::Rbx) as i32, -15);
        assert!(
            emu.cpu.flags.contains(RFlags::CF) && emu.cpu.flags.contains(RFlags::OF),
            "le produit tronqué doit lever CF et OF"
        );
    }

    /// Une division par zéro est l'exception du processeur, pas un
    /// résultat inventé — et l'adresse fautive vient avec.
    #[test]
    fn dividing_by_zero_is_reported_as_the_processor_would() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rax, 5
    xor rdx, rdx
    xor rbx, rbx
    div rbx
    hlt
",
            &[],
        );
        let err = emu.run(50).expect_err("division par zéro");
        assert!(
            matches!(err, EmuError::DivideByZero { .. }),
            "obtenu {err:?}"
        );
        // Et le quotient qui déborde est une faute distincte.
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rdx, 1          ; dividende = 2^64, quotient trop grand
    xor rax, rax
    mov rbx, 1
    div rbx
    hlt
",
            &[],
        );
        let err = emu.run(50).expect_err("quotient hors bornes");
        assert!(
            matches!(err, EmuError::DivideOverflow { .. }),
            "obtenu {err:?}"
        );
    }

    /// Décalages : le résultat, et le dernier bit sorti dans CF.
    #[test]
    fn shifts_move_the_bits_and_the_carry() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 5
    shl rax, 3          ; 40
    mov rbx, rax

    mov rcx, 0x81
    shr rcx, 1          ; 0x40, CF = 1 (le bit sorti)
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 40, "5 << 3");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0x40);
        assert!(emu.cpu.flags.contains(RFlags::CF), "le bit 0 sort dans CF");
    }

    /// `sar` recopie le bit de signe : c'est ce qui en fait une division
    /// signée par une puissance de deux, là où `shr` ferait entrer des
    /// zéros et changerait le signe.
    #[test]
    fn sar_keeps_the_sign_where_shr_loses_it() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, -16
    sar rax, 2          ; -4
    mov rbx, rax

    mov rcx, -16
    shr rcx, 2          ; un très grand nombre positif
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx) as i64, -4);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), (-16i64 as u64) >> 2);
    }

    /// `movzx` complète par des zéros, `movsx` par le bit de signe : la
    /// même valeur d'octet donne deux nombres différents, et c'est
    /// exactement la leçon.
    #[test]
    fn movzx_and_movsx_disagree_on_purpose() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov al, 0xf0
    movzx rbx, al       ; 240
    mov al, 0xf0
    movsx rcx, al       ; -16
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 240);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx) as i64, -16);
    }

    /// `setcc` met un booléen dans un octet, sans branchement — et
    /// `neg`/`not`, que le complément à deux réclame dès le premier cours.
    #[test]
    fn setcc_neg_and_not_do_their_arithmetic() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 3
    cmp rax, 3
    sete bl             ; 1
    cmp rax, 4
    setg cl             ; 0

    mov rdx, 5
    neg rdx             ; -5
    mov rsi, 0
    not rsi             ; tous les bits à 1
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rbx), 1, "3 == 3");
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rcx), 0, "3 > 4 est faux");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx) as i64, -5);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), u64::MAX);
    }

    /// `loop`, `xchg` et `leave` : la boucle idiomatique, l'échange, et
    /// le démontage du cadre de pile qu'un prologue a monté.
    #[test]
    fn loop_xchg_and_leave_behave_as_taught() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rcx, 4
    xor rax, rax
somme:
    add rax, rcx
    loop somme          ; 4+3+2+1 = 10

    mov rbx, 1
    mov rdx, 2
    xchg rbx, rdx

    push rbp
    mov rbp, rsp
    sub rsp, 32
    leave               ; RSP et RBP reviennent où ils étaient
    hlt
",
            &[],
        );
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rax),
            10,
            "la boucle a bien tourné"
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 2, "échangés");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 1);
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rsp),
            BASE + 0x8000,
            "`leave` défait le prologue en entier, `push rbp` compris"
        );
    }

    /// `adc` additionne la retenue : c'est ainsi qu'on additionne plus
    /// large qu'un registre.
    #[test]
    fn adc_carries_across_registers() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, -1         ; 0xffff_ffff_ffff_ffff
    mov rbx, 1
    add rax, rbx        ; déborde : CF = 1
    mov rcx, 0
    adc rcx, 0          ; la retenue remonte
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 1, "la retenue est passée");
    }

    /// Un `read` dont le tampon tombe hors de la fenêtre mémoire échoue —
    /// et l'entrée déjà fournie doit rester dans la file. `step` rembobine
    /// RIP pour rejouer le `syscall` à l'identique ; si les octets avaient
    /// été consommés au passage, l'utilisateur devait retaper ce qu'il
    /// venait d'envoyer, et le rejeu se bloquait sur une file vide.
    #[test]
    fn a_read_that_faults_keeps_the_input_it_could_not_write() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, 0xdeadbeef     ; tampon hors de la fenêtre simulée
    mov rdx, 4
    syscall
    hlt
",
            &[],
        );
        emu.provide_input(b"oui\n");

        let err = emu.run(100).expect_err("le tampon est hors bornes");
        assert!(matches!(err, EmuError::Mem(_)), "obtenu {err:?}");
        assert_eq!(
            emu.pending_input(),
            4,
            "l'entrée doit être intacte pour le rejeu"
        );
        // Et le `syscall` est toujours sous RIP, prêt à être rejoué.
        assert_eq!(
            emu.cpu.mem.bytes_at(emu.cpu.regs.rip, 2).unwrap(),
            &[0x0f, 0x05],
            "RIP doit être resté sur le syscall"
        );
    }

    /// Un `pop` dont l'écriture échoue laisse RSP là où il était. Il
    /// avançait avant l'écriture : chaque rejeu — que `step` provoque en
    /// rembobinant RIP — dépilait un mot de plus, et la pile se dévidait
    /// silencieusement, huit octets par tentative.
    #[test]
    fn a_failed_pop_leaves_rsp_where_it_was() {
        let mut mem = namsdec_core::Memory::new(0x400000, 0x1000);
        // `pop qword [rax]` (8F 00), avec RAX hors de la fenêtre.
        mem.load(0x400000, &[0x8f, 0x00]).unwrap();
        let mut cpu = namsdec_core::Cpu::new(mem);
        cpu.regs.rip = 0x400000;
        cpu.regs.set64(Reg64::Rsp, 0x400800);
        cpu.regs.set64(Reg64::Rax, 0xdeadbeef);
        let mut emu = Emulator::new(cpu);

        for attempt in 1..=3 {
            emu.step().expect_err("la destination est hors bornes");
            assert_eq!(
                emu.cpu.regs.get64(Reg64::Rsp),
                0x400800,
                "RSP a bougé à la tentative {attempt}"
            );
            assert_eq!(emu.cpu.regs.rip, 0x400000, "et RIP aussi");
        }
    }

    fn load_only(asm: &str, breakpoints: &[u64]) -> Emulator {
        let out = namsdec_asm::assemble(asm, BASE).expect("assemblage (nasm et ld requis)");
        let mut mem = Memory::new(BASE, 0x10000);
        // L'ELF dit où il veut être chargé et par où commencer : sur un
        // programme à plusieurs sections, ce ne sont pas la même adresse.
        mem.load(out.load_addr, &out.bytes).unwrap();

        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = out.entry;
        cpu.regs.set64(Reg64::Rsp, BASE + 0x8000);

        let mut emu = Emulator::new(cpu);
        emu.breakpoints.extend(breakpoints.iter().copied());
        emu
    }

    /// Comme `run_program`, mais sans exécuter quoi que ce soit — pour les
    /// tests qui doivent piloter `step`/`step_over` à la main. Renvoie
    /// aussi la sortie d'assemblage pour retrouver les adresses par ligne.
    fn load_program(asm: &str) -> (Emulator, namsdec_asm::AssembleOutput) {
        let out = namsdec_asm::assemble(asm, BASE).expect("assemblage (nasm et ld requis)");
        let mut mem = Memory::new(BASE, 0x10000);
        mem.load(out.load_addr, &out.bytes).unwrap();

        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = out.entry;
        cpu.regs.set64(Reg64::Rsp, BASE + 0x8000);

        (Emulator::new(cpu), out)
    }

    const SUM_LOOP_ASM: &str = "\
BITS 64
section .text
    mov rax, 0
    mov rcx, 5
sum_loop:
    add rax, rcx
    dec rcx
    jnz sum_loop
    hlt
";

    #[test]
    fn sum_loop_computes_15_and_halts() {
        let emu = run_program(SUM_LOOP_ASM, &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 15);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0);
        assert!(emu.cpu.flags.contains(RFlags::ZF));
    }

    #[test]
    fn breakpoint_stops_before_halt() {
        // sum_loop: est à 0x40000A (voir namsdec-asm::tests::assembles_sum_loop_*)
        let emu = run_program(SUM_LOOP_ASM, &[0x40000A]);
        assert!(!emu.cpu.halted);
        assert_eq!(emu.cpu.regs.rip, 0x40000A);
    }

    #[test]
    fn push_pop_round_trip() {
        let asm = "\
BITS 64
section .text
    mov rax, 0x1234
    push rax
    mov rax, 0
    pop rbx
    hlt
";
        let emu = run_program(asm, &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 0x1234);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
    }

    #[test]
    fn lea_computes_address_without_touching_memory() {
        let asm = "\
BITS 64
section .text
    mov rbx, 0x400000
    lea rax, [rbx + 0x10]
    hlt
";
        let emu = run_program(asm, &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0x400010);
    }

    #[test]
    fn inc_dec_preserve_carry_flag() {
        let asm = "\
BITS 64
section .text
    mov rax, 0xFFFFFFFFFFFFFFFF
    add rax, 2       ; positionne CF=1
    inc rbx          ; ne doit pas modifier CF
    hlt
";
        let emu = run_program(asm, &[]);
        assert!(emu.cpu.halted);
        assert!(emu.cpu.flags.contains(RFlags::CF));
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 1);
    }

    const CALL_RET_ASM: &str = "\
BITS 64
section .text
    mov rax, 1
    call add_five
    hlt
add_five:
    add rax, 5
    ret
";

    #[test]
    fn call_then_ret_round_trips_and_updates_rax() {
        let emu = run_program(CALL_RET_ASM, &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
    }

    #[test]
    fn step_into_a_call_enters_the_subroutine() {
        let (mut emu, out) = load_program(CALL_RET_ASM);
        let call_addr = out.line_to_addr[&4];
        while emu.cpu.regs.rip != call_addr {
            emu.step().unwrap();
        }
        let subroutine_addr = out.line_to_addr[&7]; // "add rax, 5"
        emu.step().unwrap();
        assert_eq!(emu.cpu.regs.rip, subroutine_addr);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 1); // pas encore exécuté
    }

    #[test]
    fn step_over_a_call_runs_the_whole_subroutine() {
        let (mut emu, out) = load_program(CALL_RET_ASM);
        let call_addr = out.line_to_addr[&4];
        while emu.cpu.regs.rip != call_addr {
            emu.step().unwrap();
        }
        let after_call_addr = out.line_to_addr[&5]; // hlt
        let result = emu.step_over(1000).unwrap();
        assert_eq!(result, StepOverResult::SteppedOverCall);
        assert_eq!(emu.cpu.regs.rip, after_call_addr);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
    }

    #[test]
    fn step_over_a_non_call_instruction_behaves_like_step() {
        let (mut emu, _out) = load_program(SUM_LOOP_ASM);
        let result = emu.step_over(1000).unwrap();
        assert_eq!(result, StepOverResult::Stepped);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0); // "mov rax, 0" tout juste exécuté
    }
}
