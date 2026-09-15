//! Interpréteur x86-64 "maison" : le décodage octets -> instruction
//! structurée est délégué à `namsdec-disasm` (donc à `iced-x86`), mais toute
//! la sémantique d'exécution (effets sur registres/flags/mémoire) est
//! écrite ici, à la main, pour rester transparente et pédagogique.

mod regmap;

use std::collections::{BTreeSet, VecDeque};

use iced_x86::{Instruction, Mnemonic, OpKind, Register};
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

    fn step(&mut self) -> Result<StepResult, EmuError>;
    fn step_over(&mut self, max_steps: u32) -> Result<StepOverResult, EmuError>;
    fn run(&mut self, max_steps: u32) -> Result<RunStop, EmuError>;

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

        let bytes: Vec<u8> = self.input.drain(..take).collect();
        self.cpu.mem.write_bytes(buf, &bytes)?;
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
        self.cpu.regs.rip = instr.next_ip();
        if let Err(e) = self.execute(&instr) {
            // L'instruction n'a pas eu lieu : RIP doit rester dessus.
            // Sinon chaque tentative suivante avançait d'un cran — et
            // relancer le pas à pas après une erreur finissait par sortir
            // du code pour « exécuter » les données qui le suivent.
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
                StepResult::Halted => return Ok(StepOverResult::Halted),
                // Le `read` est au milieu de la sous-routine : on rend la
                // main, et `step_over` sera relancé quand l'entrée sera
                // là. La sous-routine reprendra donc là où elle en est,
                // et non depuis son début.
                StepResult::NeedsInput => return Ok(StepOverResult::NeedsInput),
                StepResult::Continued => {}
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
                self.cpu.mem.write_u64(new_rsp, v)?;
                self.cpu.regs.set64(namsdec_core::Reg64::Rsp, new_rsp);
            }
            M::Pop => {
                let rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);
                let v = self.cpu.mem.read_u64(rsp)?;
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rsp.wrapping_add(8));
                self.write_op(instr, 0, v)?;
            }
            M::Lea => {
                let addr = self.effective_address(instr, 1)?;
                self.write_op(instr, 0, addr)?;
            }
            M::Nop => {}
            M::Jmp => {
                self.cpu.regs.rip = self.branch_target(instr)?;
            }
            M::Je | M::Jne | M::Jl | M::Jle | M::Jg | M::Jge | M::Jb | M::Jbe | M::Ja | M::Jae => {
                if self.condition_met(instr.mnemonic()) {
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
                self.cpu.mem.write_u64(new_rsp, ret_addr)?;
                self.cpu.regs.set64(namsdec_core::Reg64::Rsp, new_rsp);
                self.cpu.regs.rip = target;
            }
            M::Ret => {
                let rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);
                let ret_addr = self.cpu.mem.read_u64(rsp)?;
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rsp.wrapping_add(8));
                self.cpu.regs.rip = ret_addr;
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

    fn condition_met(&self, mnemonic: Mnemonic) -> bool {
        let f = &self.cpu.flags;
        let zf = f.contains(RFlags::ZF);
        let sf = f.contains(RFlags::SF);
        let cf = f.contains(RFlags::CF);
        let of = f.contains(RFlags::OF);
        match mnemonic {
            Mnemonic::Je => zf,
            Mnemonic::Jne => !zf,
            Mnemonic::Jb => cf,
            Mnemonic::Jae => !cf,
            Mnemonic::Jbe => cf || zf,
            Mnemonic::Ja => !cf && !zf,
            Mnemonic::Jl => sf != of,
            Mnemonic::Jge => sf == of,
            Mnemonic::Jle => zf || (sf != of),
            Mnemonic::Jg => !zf && (sf == of),
            _ => unreachable!("condition_met appelé pour un mnémonique non conditionnel"),
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
            .virtual_address(op_index, 0, |register, _idx, _size| {
                if register == Register::None {
                    Some(0)
                } else if register == Register::RIP {
                    Some(next_ip)
                } else {
                    map_register(register)
                        .ok()
                        .map(|(r, _)| self.cpu.regs.get64(r))
                }
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
