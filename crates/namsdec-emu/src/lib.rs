//! Interpréteur x86-64 "maison" : le décodage octets -> instruction
//! structurée est délégué à `namsdec-disasm` (donc à `iced-x86`), mais toute
//! la sémantique d'exécution (effets sur registres/flags/mémoire) est
//! écrite ici, à la main, pour rester transparente et pédagogique.

mod regmap;
mod vector;
mod x87;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use iced_x86::{ConditionCode, Instruction, Mnemonic, OpKind, Register};
use namsdec_core::{
    Access, Cpu, Memory, RFlags, Reg64, Registers, Width, add_with_flags, logic_flags,
    sub_with_flags,
};
use thiserror::Error;

pub mod explain;

use regmap::{is_high_byte, map_register};

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

/// L'adresse arrondie au début de la page suivante — elle-même si elle
/// est déjà alignée.
fn page_up(addr: u64) -> u64 {
    addr.div_ceil(PAGE).saturating_mul(PAGE)
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
    /// Les registres que l'appelé doit rendre intacts, tels qu'ils
    /// étaient au moment de l'appel.
    ///
    /// C'est la moitié de la convention d'appel que personne ne vérifie :
    /// l'autre — par où passent les arguments — se lit dans les
    /// registres, celle-ci ne se voit qu'en comparant avant et après.
    preserved: [u64; PRESERVED.len()],
}

/// Les registres que l'ABI System V confie à l'appelé : il peut s'en
/// servir, à condition de les rendre comme il les a trouvés.
pub const PRESERVED: [(&str, Reg64); 6] = [
    ("RBX", Reg64::Rbx),
    ("RBP", Reg64::Rbp),
    ("R12", Reg64::R12),
    ("R13", Reg64::R13),
    ("R14", Reg64::R14),
    ("R15", Reg64::R15),
];

/// Les registres par lesquels passent les six premiers arguments, dans
/// l'ordre.
pub const ARGUMENTS: [(&str, Reg64); 6] = [
    ("RDI", Reg64::Rdi),
    ("RSI", Reg64::Rsi),
    ("RDX", Reg64::Rdx),
    ("RCX", Reg64::Rcx),
    ("R8", Reg64::R8),
    ("R9", Reg64::R9),
];

/// Une entorse à la convention d'appel, relevée au retour d'une
/// sous-routine.
///
/// Ce n'est pas une faute d'exécution — le programme continue, et un
/// programme qui n'appelle personne d'autre que lui-même peut très bien
/// s'en passer. C'est une remarque : le jour où cette routine sera
/// appelée par du code écrit par quelqu'un d'autre, elle cassera.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbiBreach {
    /// D'où partait l'appel.
    pub call_site: u64,
    /// La sous-routine fautive.
    pub entry: u64,
    /// Ce qu'elle n'a pas rendu : un registre à préserver, ou « RSP »
    /// quand la pile ne revient pas où elle était.
    pub register: &'static str,
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
    /// Le tas d'avant : un `brk` se défait comme le reste.
    heap: Heap,
    /// Le compteur d'instructions d'avant : `rdtsc` doit rendre deux fois
    /// la même valeur si on rejoue deux fois le même pas.
    ticks: u64,
    /// Combien d'entorses avaient été relevées : revenir en arrière sur
    /// un `ret` doit retirer celle qu'il a signalée.
    breaches: usize,
}

/// Combien de pas on peut défaire. Au-delà, les plus anciens sont
/// oubliés : revenir mille instructions en arrière n'a pas de sens
/// pédagogique, et la mémoire n'est pas extensible.
const HISTORY_LIMIT: usize = 5_000;

/// Ce que fait une instruction de chaîne d'un élément.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StringOp {
    /// `movs` : recopie de `[rsi]` vers `[rdi]`.
    Move,
    /// `stos` : écrit l'accumulateur en `[rdi]`.
    Store,
    /// `lods` : lit `[rsi]` dans l'accumulateur.
    Load,
    /// `scas` : compare l'accumulateur à `[rdi]`.
    Scan,
    /// `cmps` : compare `[rsi]` à `[rdi]`.
    Compare,
}

/// La largeur d'un élément, lue sur le mnémonique.
fn string_width(mnemonic: Mnemonic) -> Option<Width> {
    use Mnemonic as M;
    Some(match mnemonic {
        M::Movsb | M::Stosb | M::Lodsb | M::Scasb | M::Cmpsb => Width::W8,
        M::Movsw | M::Stosw | M::Lodsw | M::Scasw | M::Cmpsw => Width::W16,
        M::Movsd | M::Stosd | M::Lodsd | M::Scasd | M::Cmpsd => Width::W32,
        M::Movsq | M::Stosq | M::Lodsq | M::Scasq | M::Cmpsq => Width::W64,
        _ => return None,
    })
}

/// `movsd` et `cmpsd` nomment deux instructions sans rapport : celle qui
/// travaille sur une chaîne, et celle du jeu SSE qui manipule un flottant
/// dans un registre `xmm`. Seule la première désigne ses opérandes par
/// RSI/RDI, et c'est à ça qu'on les distingue.
fn is_string_operation(instr: &Instruction) -> bool {
    (0..instr.op_count()).any(|op| {
        matches!(
            instr.op_kind(op),
            OpKind::MemorySegRSI
                | OpKind::MemorySegESI
                | OpKind::MemorySegSI
                | OpKind::MemoryESRDI
                | OpKind::MemoryESEDI
                | OpKind::MemoryESDI
        )
    })
}

/// L'arrondi au plus proche, l'impair tranché vers le pair — le mode par
/// défaut du processeur, celui que `cvtsd2si` applique.
fn round_even(value: f64) -> f64 {
    let rounded = value.round();
    if (value - value.trunc()).abs() == 0.5 && rounded % 2.0 != 0.0 {
        rounded - value.signum()
    } else {
        rounded
    }
}

/// Le résultat entier d'une conversion. Hors bornes ou NaN, le
/// processeur rend « l'entier indéfini » : le plus petit négatif de la
/// largeur demandée. Une valeur reconnaissable, plutôt qu'un repli
/// silencieux sur zéro.
fn to_integer(value: f64, w: Width) -> u64 {
    let (min, max, indefinite) = match w {
        Width::W64 => (i64::MIN as f64, i64::MAX as f64, i64::MIN as u64),
        _ => (
            f64::from(i32::MIN),
            f64::from(i32::MAX),
            u64::from(i32::MIN as u32),
        ),
    };
    if value.is_nan() || value < min || value > max {
        indefinite
    } else {
        value as i64 as u64
    }
}

/// Le sens d'une rotation, et si elle passe par la retenue — auquel cas
/// le registre tourne sur un bit de plus que sa largeur.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RotateKind {
    Left,
    Right,
    CarryLeft,
    CarryRight,
}

/// Ce qu'on fait au bit désigné, une fois qu'on l'a lu dans CF.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BitOp {
    /// `bt` : le lire, et rien d'autre.
    Test,
    Set,
    Reset,
    Complement,
}

impl BitOp {
    /// La valeur à réécrire, ou `None` quand il n'y a rien à écrire.
    fn apply(self, value: u64, bit: u32) -> Option<u64> {
        let one = 1u64 << bit;
        match self {
            BitOp::Test => None,
            BitOp::Set => Some(value | one),
            BitOp::Reset => Some(value & !one),
            BitOp::Complement => Some(value ^ one),
        }
    }
}

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
/// Lit une valeur pour la *montrer* — un point de surveillance, un
/// vidage. Sans contrôle de droits : regarder n'est pas accéder.
fn read_bytes(mem: &namsdec_core::Memory, addr: u64, len: usize) -> u64 {
    let mut value = 0u64;
    for i in (0..len).rev() {
        let byte = mem.peek_u8(addr.wrapping_add(i as u64)).unwrap_or(0);
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
    /// Un registre surveillé a changé de valeur.
    ///
    /// La mémoire n'est pas le seul endroit où l'on attend quelque
    /// chose : « arrête-toi quand RAX change » est la question qu'on se
    /// pose le plus souvent en apprenant, et elle n'a pas de réponse dans
    /// un point d'arrêt — on ne sait pas *où* la poser.
    RegisterChanged {
        register: &'static str,
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

    /// Les registres à surveiller : l'exécution s'arrête quand l'un
    /// d'eux change.
    fn set_watched_registers(&mut self, registers: &[(&'static str, Reg64)]);
    fn watched_registers(&self) -> &[(&'static str, Reg64, u64)];

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

    /// Le tas : où il commence, où `brk` l'a laissé, jusqu'où il peut
    /// monter. Un moteur qui n'en simule pas rend un tas vide.
    fn heap(&self) -> Heap;

    /// Ce que les sous-routines revenues n'ont pas rendu comme elles
    /// l'avaient trouvé. Vide pour un moteur qui ne regarde pas.
    fn breaches(&self) -> &[AbiBreach];

    /// Combien d'instructions ont été exécutées depuis le départ.
    ///
    /// La mesure la plus simple de ce que coûte un programme : deux
    /// façons d'écrire la même boucle ne donnent pas le même nombre.
    fn executed(&self) -> u64;

    /// Combien de fois chaque adresse a été exécutée depuis le départ.
    ///
    /// C'est le profil du programme : il dit ce qui a tourné cinq fois
    /// et ce qui n'a jamais tourné du tout — la boucle qu'on croyait
    /// écrire, la branche qu'on croyait prendre.
    fn visits(&self) -> &BTreeMap<u64, u64>;

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

/// La taille d'une page. `brk` travaille par pages sous Linux, et
/// aligner la rupture de départ dessus, c'est montrer la même chose ici.
const PAGE: u64 = 0x1000;

/// Le tas : le segment que `brk` fait grandir, entre la fin de l'image du
/// programme et la pile.
///
/// Il n'y a pas de pagination ici, ni de noyau qui cherche de la place :
/// la mémoire simulée existe déjà tout entière. Ce que `brk` déplace est
/// donc une simple frontière — mais c'est exactement ce qu'un programme
/// voit d'elle, et c'est ce qu'il faut comprendre pour écrire un
/// allocateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Heap {
    start: u64,
    current: u64,
    limit: u64,
}

impl Heap {
    /// La première adresse du tas. `brk` ne descend jamais en dessous :
    /// ce qui est là, c'est le programme lui-même.
    pub fn start(&self) -> u64 {
        self.start
    }

    /// La rupture courante : la première adresse **hors** du tas. C'est
    /// ce que rend `brk(0)`.
    pub fn current(&self) -> u64 {
        self.current
    }

    /// Le plafond : au-delà, `brk` échoue. Il laisse de quoi vivre à la
    /// pile, qui descend depuis le haut de la mémoire.
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Combien d'octets le programme s'est fait donner.
    pub fn len(&self) -> u64 {
        self.current - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.current == self.start
    }
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
    /// Les registres surveillés, avec la valeur vue au dernier armement.
    watched_registers: Vec<(&'static str, Reg64, u64)>,
    /// Le tas, tel que `brk` l'a laissé.
    heap: Heap,
    /// Les entorses à la convention d'appel relevées jusqu'ici.
    breaches: Vec<AbiBreach>,
    /// Combien d'instructions ont été exécutées depuis le chargement.
    /// C'est ce que rend `rdtsc` : le temps qu'on peut mesurer ici.
    ticks: u64,
    /// Combien de fois chaque adresse a été exécutée.
    visits: BTreeMap<u64, u64>,
}

impl Backend for Emulator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn executed(&self) -> u64 {
        self.ticks
    }

    fn visits(&self) -> &BTreeMap<u64, u64> {
        &self.visits
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

    fn set_watched_registers(&mut self, registers: &[(&'static str, Reg64)]) {
        Emulator::set_watched_registers(self, registers);
    }

    fn watched_registers(&self) -> &[(&'static str, Reg64, u64)] {
        Emulator::watched_registers(self)
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

    fn heap(&self) -> Heap {
        Emulator::heap(self)
    }

    fn breaches(&self) -> &[AbiBreach] {
        Emulator::breaches(self)
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
        let end = cpu.mem.range().end;
        Self {
            breakpoints: BTreeSet::new(),
            output: Vec::new(),
            input: VecDeque::new(),
            input_closed: false,
            exit_code: None,
            history: VecDeque::new(),
            frames: Vec::new(),
            watchpoints: Vec::new(),
            watched_registers: Vec::new(),
            // Pas de tas tant que personne n'a dit où le mettre : seul
            // celui qui a chargé le programme sait où son image finit.
            // `brk` se comporte alors comme sous un système à court de
            // mémoire — il rend la rupture sans la déplacer.
            heap: Heap {
                start: end,
                current: end,
                limit: end,
            },
            ticks: 0,
            visits: BTreeMap::new(),
            breaches: Vec::new(),
            cpu,
        }
    }

    /// Installe le tas : `start` est la fin de l'image du programme (la
    /// rupture part de la page suivante, comme sous Linux), `limit` le
    /// plus haut qu'on le laisse monter.
    ///
    /// C'est à l'appelant de le dire parce que lui seul sait où le
    /// programme finit et où sa pile commence ; l'émulateur ne voit qu'un
    /// bloc d'octets.
    pub fn set_heap(&mut self, start: u64, limit: u64) {
        let range = self.cpu.mem.range();
        let start = page_up(start).clamp(range.start, range.end);
        // Un plafond plus bas que le départ n'est pas une erreur : c'est
        // un tas de taille nulle, et `brk` refusera tout.
        let limit = limit.clamp(start, range.end);
        self.heap = Heap {
            start,
            current: start,
            limit,
        };
        self.map_heap();
    }

    /// Le tas, tel que `brk` l'a laissé.
    pub fn heap(&self) -> Heap {
        self.heap
    }

    /// Déplace la rupture et rend celle qui vaut désormais.
    ///
    /// Linux rend **toujours** la rupture effective, pas un code
    /// d'erreur : inchangée, elle veut dire « refusé ». D'où l'usage de
    /// `brk(0)`, qui ne demande rien et sert donc à la lire.
    fn set_break(&mut self, requested: u64) -> u64 {
        if (self.heap.start..=self.heap.limit).contains(&requested) {
            if requested < self.heap.current {
                // Ce qu'on rend au système est perdu : le noyau reprend
                // les pages et n'en rendra que des vierges. Les remettre
                // à zéro ici, c'est rendre cet oubli visible plutôt que
                // de laisser un programme fautif retrouver ses données.
                let len = (self.heap.current - requested) as usize;
                self.record_memory(requested, len);
                let _ = self.cpu.mem.poke(requested, &vec![0u8; len]);
            }
            self.heap.current = requested;
            // La zone projetée suit la rupture : au-delà, il n'y a rien,
            // et y toucher est une faute — exactement ce qui arrive quand
            // on oublie de demander la place avant de s'en servir.
            self.map_heap();
        }
        self.heap.current
    }

    /// (Re)déclare la zone du tas d'après la rupture courante.
    ///
    /// Seulement si la mémoire est déjà découpée en zones : déclarer
    /// celle du tas dans un bloc qui n'en a aucune rendrait tout le reste
    /// — le programme lui-même — inadressable. Les droits sont une
    /// décision de celui qui charge, pas un effet de bord de `brk`.
    fn map_heap(&mut self) {
        if !self.cpu.mem.has_regions() {
            return;
        }
        self.cpu.mem.map(
            self.heap.start..self.heap.current,
            namsdec_core::Perms::DATA,
            "heap",
        );
    }

    /// L'appel système Linux x86-64 : le numéro est dans RAX, les
    /// arguments dans RDI, RSI, RDX (puis R10, R8, R9 — dont aucun appel
    /// géré ici n'a besoin), et la valeur de retour repart dans RAX.
    ///
    /// Quatre appels seulement sont implémentés : ceux dont un premier
    /// programme en assembleur a besoin pour faire quelque chose de
    /// visible — `read`, `write`, `exit` — et `brk`, qui lui donne de la
    /// mémoire. namsdec n'est pas un noyau, et ne prétend pas l'être : un
    /// appel non géré s'arrête en le disant, ce qui apprend davantage
    /// qu'un code d'erreur silencieux.
    fn syscall(&mut self) -> Result<(), EmuError> {
        const SYS_READ: u64 = 0;
        const SYS_WRITE: u64 = 1;
        const SYS_EXIT: u64 = 60;
        /// `exit_group` termine tous les fils d'exécution. Sans threads,
        /// c'est un `exit` — et c'est celui que produisent la plupart des
        /// bibliothèques C.
        const SYS_EXIT_GROUP: u64 = 231;
        /// `brk` déplace la fin du tas. C'est sur lui que repose tout
        /// `malloc`, et c'est le seul appel « mémoire » qu'un programme
        /// écrit à la main utilise sans bibliothèque.
        const SYS_BRK: u64 = 12;

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
            SYS_BRK => {
                let requested = self.cpu.regs.get64(Reg64::Rdi);
                let now = self.set_break(requested);
                self.cpu.regs.set64(Reg64::Rax, now);
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
        self.ticks = self.ticks.wrapping_add(1);
        // Un compteur par adresse, pas par pas : la carte ne grandit
        // jamais au-delà du nombre d'instructions du programme, quelle
        // que soit la durée de l'exécution.
        *self.visits.entry(rip_before).or_insert(0) += 1;
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
            if let Some((register, old, new)) = self.changed_register() {
                return Ok(RunStop::RegisterChanged { register, old, new });
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

            // Le drapeau de direction : vers les adresses croissantes
            // (`cld`, l'état normal) ou décroissantes (`std`).
            M::Cld => self.cpu.flags.remove(RFlags::DF),
            M::Std => self.cpu.flags.insert(RFlags::DF),

            // Les instructions de chaînes. Le garde `is_string_operation`
            // écarte l'homonyme SSE : `movsd xmm0, [rax]` porte le même
            // mnémonique que `movsd` sur chaîne, et n'a rien à voir.
            M::Movsb | M::Movsw | M::Movsd | M::Movsq if is_string_operation(instr) => {
                self.string_step(instr, StringOp::Move)?
            }
            M::Stosb | M::Stosw | M::Stosd | M::Stosq => {
                self.string_step(instr, StringOp::Store)?
            }
            M::Lodsb | M::Lodsw | M::Lodsd | M::Lodsq => self.string_step(instr, StringOp::Load)?,
            M::Scasb | M::Scasw | M::Scasd | M::Scasq => self.string_step(instr, StringOp::Scan)?,
            M::Cmpsb | M::Cmpsw | M::Cmpsd | M::Cmpsq if is_string_operation(instr) => {
                self.string_step(instr, StringOp::Compare)?
            }
            // --- Changer une valeur signée de largeur -----------------
            // `cbw`/`cwde`/`cdqe` recopient le bit de signe dans la
            // moitié haute ; `cwd`/`cdq`/`cqo` le recopient dans le
            // registre voisin, ce qu'il faut faire avant chaque `idiv`
            // sous peine de diviser par un reste oublié.
            M::Cbw => {
                let v = sign_extend(u64::from(self.cpu.regs.get8l(Reg64::Rax)), Width::W8);
                self.cpu.regs.set16(Reg64::Rax, v as u16);
            }
            M::Cwde => {
                let v = sign_extend(u64::from(self.cpu.regs.get16(Reg64::Rax)), Width::W16);
                self.cpu.regs.set32(Reg64::Rax, v as u32);
            }
            M::Cdqe => {
                let v = sign_extend(u64::from(self.cpu.regs.get32(Reg64::Rax)), Width::W32);
                self.cpu.regs.set64(Reg64::Rax, v);
            }
            M::Cwd => {
                let high = if (self.cpu.regs.get16(Reg64::Rax) as i16) < 0 {
                    u16::MAX
                } else {
                    0
                };
                self.cpu.regs.set16(Reg64::Rdx, high);
            }
            M::Cdq => {
                let high = if (self.cpu.regs.get32(Reg64::Rax) as i32) < 0 {
                    u32::MAX
                } else {
                    0
                };
                self.cpu.regs.set32(Reg64::Rdx, high);
            }
            M::Cqo => {
                let high = if (self.cpu.regs.get64(Reg64::Rax) as i64) < 0 {
                    u64::MAX
                } else {
                    0
                };
                self.cpu.regs.set64(Reg64::Rdx, high);
            }

            // --- Rotations -------------------------------------------
            M::Rol => self.rotate(instr, RotateKind::Left)?,
            M::Ror => self.rotate(instr, RotateKind::Right)?,
            M::Rcl => self.rotate(instr, RotateKind::CarryLeft)?,
            M::Rcr => self.rotate(instr, RotateKind::CarryRight)?,

            // --- Décalages doubles -----------------------------------
            M::Shld => self.double_shift(instr, true)?,
            M::Shrd => self.double_shift(instr, false)?,

            // --- Le bit désigné --------------------------------------
            M::Bt => self.bit_test(instr, BitOp::Test)?,
            M::Bts => self.bit_test(instr, BitOp::Set)?,
            M::Btr => self.bit_test(instr, BitOp::Reset)?,
            M::Btc => self.bit_test(instr, BitOp::Complement)?,

            // --- Chercher et compter des bits ------------------------
            M::Bsf | M::Bsr => {
                let w = self.dest_width(instr)?;
                let src = self.read_op(instr, 1)? & w.mask();
                let mut flags = self.cpu.flags;
                flags.set(RFlags::ZF, src == 0);
                self.cpu.flags = flags;
                // Source nulle : la destination garde sa valeur — c'est
                // « indéfini » chez Intel, et ne rien faire est la seule
                // lecture qui ne mente pas.
                if src != 0 {
                    let index = if instr.mnemonic() == M::Bsf {
                        src.trailing_zeros()
                    } else {
                        63 - src.leading_zeros()
                    };
                    self.write_op(instr, 0, u64::from(index))?;
                }
            }
            M::Tzcnt | M::Lzcnt => {
                let w = self.dest_width(instr)?;
                let bits = w.bits();
                let src = self.read_op(instr, 1)? & w.mask();
                // Là où `bsf`/`bsr` laissent la destination en l'air,
                // `tzcnt`/`lzcnt` rendent la largeur de l'opérande : une
                // réponse, et non un cas particulier à traiter.
                let count = if src == 0 {
                    bits
                } else if instr.mnemonic() == M::Tzcnt {
                    src.trailing_zeros()
                } else {
                    src.leading_zeros() - (64 - bits)
                };
                let mut flags = self.cpu.flags;
                flags.set(RFlags::CF, src == 0);
                flags.set(RFlags::ZF, count == 0);
                self.cpu.flags = flags;
                self.write_op(instr, 0, u64::from(count))?;
            }
            M::Popcnt => {
                let w = self.dest_width(instr)?;
                let src = self.read_op(instr, 1)? & w.mask();
                let count = u64::from(src.count_ones());
                let mut flags = RFlags::empty();
                flags.set(RFlags::ZF, count == 0);
                self.cpu.flags.apply_arith(flags);
                self.write_op(instr, 0, count)?;
            }

            // --- Échanges -------------------------------------------
            M::Xadd => {
                let w = self.dest_width(instr)?;
                let dest = self.read_op(instr, 0)?;
                let src = self.read_op(instr, 1)?;
                let (result, flags) = add_with_flags(dest, src, w);
                self.cpu.flags.apply_arith(flags);
                // L'ancienne destination part dans la source : c'est ce
                // qui fait de `xadd` un compteur atomique.
                self.write_op(instr, 1, dest)?;
                self.write_op(instr, 0, result)?;
            }
            M::Cmpxchg => {
                let w = self.dest_width(instr)?;
                let dest = self.read_op(instr, 0)?;
                let acc = self.accumulator(w);
                let (_, flags) = sub_with_flags(acc & w.mask(), dest & w.mask(), w);
                self.cpu.flags.apply_arith(flags);
                if acc & w.mask() == dest & w.mask() {
                    let src = self.read_op(instr, 1)?;
                    self.write_op(instr, 0, src)?;
                } else {
                    // Échec : l'accumulateur reçoit ce qu'il y avait, de
                    // quoi réessayer avec la bonne valeur de départ.
                    self.set_accumulator(w, dest);
                }
            }
            M::Bswap => {
                let w = self.dest_width(instr)?;
                let v = self.read_op(instr, 0)?;
                let swapped = match w {
                    Width::W64 => v.swap_bytes(),
                    _ => u64::from((v as u32).swap_bytes()),
                };
                self.write_op(instr, 0, swapped)?;
            }
            M::Movbe => {
                let w = self.dest_width(instr)?;
                let v = self.read_op(instr, 1)? & w.mask();
                let swapped = match w {
                    Width::W16 => u64::from((v as u16).swap_bytes()),
                    Width::W32 => u64::from((v as u32).swap_bytes()),
                    _ => v.swap_bytes(),
                };
                self.write_op(instr, 0, swapped)?;
            }

            // --- Les indicateurs à la main ---------------------------
            M::Clc => self.cpu.flags.remove(RFlags::CF),
            M::Stc => self.cpu.flags.insert(RFlags::CF),
            M::Cmc => self.cpu.flags.toggle(RFlags::CF),
            M::Lahf => {
                // Le bit 1 vaut toujours 1 sur un vrai processeur : le
                // recopier ici évite qu'un programme qui compare AH à une
                // constante trouve autre chose que sur la machine.
                let ah = (self.cpu.flags.bits() & 0xd5) as u8 | 0x02;
                self.cpu.regs.set8h(Reg64::Rax, ah);
            }
            M::Sahf => {
                let ah = u64::from(self.cpu.regs.get8h(Reg64::Rax));
                let kept = self.cpu.flags.bits() & !0xd5;
                self.cpu.flags = RFlags::from_bits_truncate(kept | (ah & 0xd5));
            }
            M::Pushfq | M::Pushfd | M::Pushf => {
                let value = self.cpu.flags.bits() | 0x02;
                self.push(value)?;
            }
            M::Popfq | M::Popfd | M::Popf => {
                let value = self.pop()?;
                self.cpu.flags = RFlags::from_bits_truncate(value);
            }

            // --- Boucles et sauts sur RCX ----------------------------
            M::Loope | M::Loopne => {
                let rcx = self.cpu.regs.get64(Reg64::Rcx).wrapping_sub(1);
                self.cpu.regs.set64(Reg64::Rcx, rcx);
                let zf = self.cpu.flags.contains(RFlags::ZF);
                let wanted = instr.mnemonic() == M::Loope;
                if rcx != 0 && zf == wanted {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }
            M::Jrcxz | M::Jecxz => {
                let counter = if instr.mnemonic() == M::Jrcxz {
                    self.cpu.regs.get64(Reg64::Rcx)
                } else {
                    u64::from(self.cpu.regs.get32(Reg64::Rcx))
                };
                if counter == 0 {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }

            // --- Cadre de pile, table, et le reste -------------------
            M::Enter => {
                // Le niveau d'imbrication (le second opérande) sert aux
                // langages à fonctions imbriquées, que personne n'écrit
                // en assembleur : seul le niveau 0 est simulé, et les
                // autres se signalent plutôt que de mentir.
                if self.read_op(instr, 1)? & 0x1f != 0 {
                    return Err(EmuError::UnsupportedInstruction {
                        backend: Self::NAME,
                        mnemonic: "enter (niveau ≠ 0)".to_owned(),
                    });
                }
                let rbp = self.cpu.regs.get64(Reg64::Rbp);
                self.push(rbp)?;
                let rsp = self.cpu.regs.get64(Reg64::Rsp);
                self.cpu.regs.set64(Reg64::Rbp, rsp);
                let frame = self.read_op(instr, 0)?;
                self.cpu.regs.set64(Reg64::Rsp, rsp.wrapping_sub(frame));
            }
            M::Xlatb => {
                let addr = self
                    .cpu
                    .regs
                    .get64(Reg64::Rbx)
                    .wrapping_add(u64::from(self.cpu.regs.get8l(Reg64::Rax)));
                let byte = self.cpu.mem.read_u8(addr)?;
                self.cpu.regs.set8l(Reg64::Rax, byte);
            }
            M::Rdtsc => {
                // Un vrai compteur de cycles n'a pas de sens ici ; celui
                // des instructions exécutées en a un : il mesure ce que
                // le programme fait, et non ce que la machine met à le
                // faire. Il avance donc de 1 par pas, toujours.
                let ticks = self.ticks;
                self.cpu.regs.set32(Reg64::Rax, ticks as u32);
                self.cpu.regs.set32(Reg64::Rdx, (ticks >> 32) as u32);
            }
            // Un point d'arrêt logiciel : l'exécution s'arrête, comme
            // devant un débogueur — sauf qu'ici, le débogueur, c'est
            // l'application elle-même.
            M::Int3 => {
                self.cpu.halted = true;
            }
            // `ud2` existe pour planter : c'est ce qu'un compilateur pose
            // là où le programme ne doit jamais passer.
            M::Ud2 => {
                return Err(EmuError::InvalidInstruction { addr: instr.ip() });
            }
            // --- Les instructions de bits « modernes » (BMI) ---------
            // Elles font en une instruction ce qui en demandait trois, et
            // surtout : elles ne touchent pas aux indicateurs, ce qui
            // permet de les glisser au milieu d'un calcul.
            M::Shlx | M::Shrx | M::Sarx => {
                let w = self.dest_width(instr)?;
                let bits = u64::from(w.bits());
                let value = self.read_op(instr, 1)? & w.mask();
                let count = self.read_op(instr, 2)? % bits;
                let result = match instr.mnemonic() {
                    M::Shlx => (value << count) & w.mask(),
                    M::Shrx => value >> count,
                    _ => ((sign_extend(value, w) as i64) >> count) as u64 & w.mask(),
                };
                self.write_op(instr, 0, result)?;
            }
            M::Rorx => {
                let w = self.dest_width(instr)?;
                let bits = w.bits();
                let value = self.read_op(instr, 1)? & w.mask();
                let count = (self.read_op(instr, 2)? % u64::from(bits)) as u32;
                let result = if count == 0 {
                    value
                } else {
                    ((value >> count) | (value << (bits - count))) & w.mask()
                };
                self.write_op(instr, 0, result)?;
            }
            M::Andn => {
                let w = self.dest_width(instr)?;
                let result = !self.read_op(instr, 1)? & self.read_op(instr, 2)? & w.mask();
                self.cpu.flags.apply_arith(logic_flags(result, w));
                self.write_op(instr, 0, result)?;
            }
            M::Blsi | M::Blsr | M::Blsmsk => {
                let w = self.dest_width(instr)?;
                let src = self.read_op(instr, 1)? & w.mask();
                // `src & -src` isole le bit bas posé ; les trois
                // instructions en dérivent.
                let lowest = src.isolate_lowest_one();
                let result = match instr.mnemonic() {
                    M::Blsi => lowest,
                    M::Blsr => src & src.wrapping_sub(1),
                    _ => src ^ src.wrapping_sub(1),
                } & w.mask();
                let mut flags = logic_flags(result, w);
                // `blsi` dit « il y avait un bit » ; les deux autres
                // disent « il n'y en avait pas ». Une différence de
                // convention, pas de calcul.
                flags.set(RFlags::CF, (src == 0) != (instr.mnemonic() == M::Blsi));
                if instr.mnemonic() == M::Blsmsk {
                    flags.remove(RFlags::ZF);
                }
                self.cpu.flags.apply_arith(flags);
                self.write_op(instr, 0, result)?;
            }
            M::Bextr | M::Bzhi => {
                let w = self.dest_width(instr)?;
                let bits = w.bits();
                let src = self.read_op(instr, 1)? & w.mask();
                let control = self.read_op(instr, 2)?;
                let result = if instr.mnemonic() == M::Bextr {
                    let start = (control & 0xff) as u32;
                    let len = ((control >> 8) & 0xff) as u32;
                    if start >= bits || len == 0 {
                        0
                    } else {
                        let taken = len.min(bits - start);
                        let mask = if taken >= 64 {
                            u64::MAX
                        } else {
                            (1u64 << taken) - 1
                        };
                        (src >> start) & mask
                    }
                } else {
                    let index = (control & 0xff) as u32;
                    if index >= bits {
                        src
                    } else {
                        src & ((1u64 << index) - 1)
                    }
                };
                let mut flags = logic_flags(result, w);
                flags.remove(RFlags::SF);
                self.cpu.flags.apply_arith(flags);
                self.write_op(instr, 0, result)?;
            }
            M::Mulx => {
                // Le seul produit qui ne touche à aucun indicateur, et qui
                // dit lui-même où ranger ses deux moitiés.
                let w = self.dest_width(instr)?;
                let src = self.read_op(instr, 2)? & w.mask();
                let acc = match w {
                    Width::W32 => u64::from(self.cpu.regs.get32(Reg64::Rdx)),
                    _ => self.cpu.regs.get64(Reg64::Rdx),
                };
                let full = u128::from(acc) * u128::from(src);
                let low = (full & u128::from(w.mask())) as u64;
                let high = ((full >> w.bits()) & u128::from(w.mask())) as u64;
                self.write_op(instr, 1, low)?;
                self.write_op(instr, 0, high)?;
            }

            // --- Ce que la machine dit d'elle-même -------------------
            M::Cpuid => {
                // namsdec n'est pas un processeur : il ne peut annoncer
                // aucune extension sans mentir. Il se nomme, et déclare
                // n'avoir rien de plus — un programme qui interroge
                // `cpuid` prendra donc son chemin de repli, ce qui est la
                // seule réponse honnête.
                let leaf = self.cpu.regs.get32(Reg64::Rax);
                let (a, b, c, d) = if leaf == 0 {
                    // « namsdec emu », dans l'ordre EBX:EDX:ECX du manuel.
                    (0u32, 0x736d_616e, 0x0075_6d65, 0x2063_6564)
                } else {
                    (0, 0, 0, 0)
                };
                self.cpu.regs.set32(Reg64::Rax, a);
                self.cpu.regs.set32(Reg64::Rbx, b);
                self.cpu.regs.set32(Reg64::Rcx, c);
                self.cpu.regs.set32(Reg64::Rdx, d);
            }
            M::Rdtscp => {
                let ticks = self.ticks;
                self.cpu.regs.set32(Reg64::Rax, ticks as u32);
                self.cpu.regs.set32(Reg64::Rdx, (ticks >> 32) as u32);
                // L'identifiant du cœur : il n'y en a qu'un.
                self.cpu.regs.set32(Reg64::Rcx, 0);
            }

            // Rien à simuler : ni cache, ni pipeline, ni exécution dans
            // le désordre. Elles restent légales, et ne font rien.
            M::Pause | M::Mfence | M::Lfence | M::Sfence | M::Endbr64 | M::Endbr32 => {}

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
                    preserved: PRESERVED.map(|(_, reg)| self.cpu.regs.get64(reg)),
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
                if let Some(frame) = self.frames.pop() {
                    self.check_convention(&frame);
                    if let Some(undo) = self.history.back_mut() {
                        undo.frames = FrameChange::Popped(frame);
                    }
                }
            }
            M::Hlt => {
                self.cpu.halted = true;
            }
            M::Syscall => self.syscall()?,
            // Le jeu entier s'arrête ici. Le vectoriel et le flottant
            // sont traités par famille, chacun dans son fichier : c'est
            // la même machine, mais ce ne sont pas les mêmes registres,
            // ni les mêmes questions.
            _ => return self.execute_vector(instr),
        }
        Ok(())
    }

    /// La condition d'un `jcc`, d'un `setcc` ou d'un `cmovcc`, lue sur
    /// l'instruction elle-même plutôt que déduite de son mnémonique :
    /// iced-x86 les ramène toutes à un même code, et les trois familles
    /// se partagent donc ce seul test.
    fn condition_met(&self, instr: &Instruction) -> bool {
        condition_met(instr.condition_code(), self.cpu.flags).expect("instruction sans condition")
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

    /// **Une seule itération** d'une instruction de chaîne, préfixe `rep`
    /// compris.
    ///
    /// C'est le choix de conception de ces instructions-là. Un vrai
    /// processeur les rend interruptibles : `rep movsb` n'est pas une
    /// boucle atomique, il avance d'un élément et se rejoue tant que RCX
    /// n'est pas nul. On fait pareil, pour deux raisons qui vont dans le
    /// même sens : le pas à pas montre RSI, RDI et RCX avancer un à un —
    /// c'est ce qu'on vient voir en apprenant —, et une copie d'un
    /// million d'octets ne fige pas l'interface le temps d'une frame.
    ///
    /// RIP a déjà avancé quand on arrive ici (voir `step`) : le rejeu
    /// consiste à le remettre sur l'instruction.
    fn string_step(&mut self, instr: &Instruction, op: StringOp) -> Result<(), EmuError> {
        use namsdec_core::Reg64::{Rcx, Rdi, Rsi};
        let w = string_width(instr.mnemonic()).ok_or_else(|| EmuError::UnsupportedInstruction {
            backend: Self::NAME,
            mnemonic: format!("{:?}", instr.mnemonic()),
        })?;
        let repeat = instr.has_rep_prefix() || instr.has_repe_prefix() || instr.has_repne_prefix();

        // `rep` avec RCX nul ne fait rien du tout — pas même une
        // itération. C'est la nuance qui distingue un compteur à zéro
        // d'un compteur à un.
        if repeat && self.cpu.regs.get64(Rcx) == 0 {
            return Ok(());
        }

        let bytes = u64::from(w.bits() / 8);
        let step = if self.cpu.flags.contains(RFlags::DF) {
            bytes.wrapping_neg()
        } else {
            bytes
        };
        let (rsi, rdi) = (self.cpu.regs.get64(Rsi), self.cpu.regs.get64(Rdi));

        match op {
            StringOp::Move => {
                let v = self.read_mem(rsi, w)?;
                self.write_mem(rdi, w, v)?;
                self.cpu.regs.set64(Rsi, rsi.wrapping_add(step));
                self.cpu.regs.set64(Rdi, rdi.wrapping_add(step));
            }
            StringOp::Store => {
                let v = self.accumulator(w);
                self.write_mem(rdi, w, v)?;
                self.cpu.regs.set64(Rdi, rdi.wrapping_add(step));
            }
            StringOp::Load => {
                let v = self.read_mem(rsi, w)?;
                self.set_accumulator(w, v);
                self.cpu.regs.set64(Rsi, rsi.wrapping_add(step));
            }
            StringOp::Scan => {
                let v = self.read_mem(rdi, w)?;
                let (_, flags) = sub_with_flags(self.accumulator(w), v, w);
                self.cpu.flags.apply_arith(flags);
                self.cpu.regs.set64(Rdi, rdi.wrapping_add(step));
            }
            StringOp::Compare => {
                let left = self.read_mem(rsi, w)?;
                let right = self.read_mem(rdi, w)?;
                let (_, flags) = sub_with_flags(left, right, w);
                self.cpu.flags.apply_arith(flags);
                self.cpu.regs.set64(Rsi, rsi.wrapping_add(step));
                self.cpu.regs.set64(Rdi, rdi.wrapping_add(step));
            }
        }

        if repeat {
            let remaining = self.cpu.regs.get64(Rcx).wrapping_sub(1);
            self.cpu.regs.set64(Rcx, remaining);
            // `repe`/`repne` s'arrêtent aussi sur la comparaison : c'est
            // ce qui fait de `repne scasb` un « cherche cet octet » et de
            // `repe cmpsb` un « compare jusqu'à la différence ».
            let zf = self.cpu.flags.contains(RFlags::ZF);
            let again = remaining != 0
                && match op {
                    StringOp::Scan | StringOp::Compare => {
                        if instr.has_repne_prefix() {
                            !zf
                        } else {
                            zf
                        }
                    }
                    _ => true,
                };
            if again {
                self.cpu.regs.rip = instr.ip();
            }
        }
        Ok(())
    }

    fn set_accumulator(&mut self, w: Width, value: u64) {
        use namsdec_core::Reg64::Rax;
        match w {
            Width::W8 => self.cpu.regs.set8l(Rax, value as u8),
            Width::W16 => self.cpu.regs.set16(Rax, value as u16),
            Width::W32 => self.cpu.regs.set32(Rax, value as u32),
            Width::W64 => self.cpu.regs.set64(Rax, value),
        }
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

    /// Empile huit octets : RSP descend, puis la valeur s'écrit.
    fn push(&mut self, value: u64) -> Result<(), EmuError> {
        let new_rsp = self.cpu.regs.get64(Reg64::Rsp).wrapping_sub(8);
        self.record_memory(new_rsp, 8);
        self.cpu.mem.write_u64(new_rsp, value)?;
        self.cpu.regs.set64(Reg64::Rsp, new_rsp);
        Ok(())
    }

    /// Dépile huit octets. RSP n'avance que si la lecture a réussi : un
    /// pas qui échoue doit pouvoir se rejouer à l'identique.
    fn pop(&mut self) -> Result<u64, EmuError> {
        let rsp = self.cpu.regs.get64(Reg64::Rsp);
        let value = self.cpu.mem.read_u64(rsp)?;
        self.cpu.regs.set64(Reg64::Rsp, rsp.wrapping_add(8));
        Ok(value)
    }

    /// Les quatre rotations. Contrairement aux décalages, elles ne
    /// touchent ni ZF, ni SF, ni PF : seule la retenue en sort, et le
    /// débordement n'a de sens que pour une rotation d'un cran.
    fn rotate(&mut self, instr: &Instruction, kind: RotateKind) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let bits = w.bits();
        let mask = w.mask();
        let a = self.read_op(instr, 0)? & mask;
        let raw = self.read_op(instr, 1)? & if bits == 64 { 0x3f } else { 0x1f };
        if raw == 0 {
            // Compte nul : pas même un indicateur ne bouge.
            return Ok(());
        }

        let through_carry = matches!(kind, RotateKind::CarryLeft | RotateKind::CarryRight);
        let count = if through_carry {
            // La retenue fait un bit de plus dans le tour.
            match bits {
                8 => raw % 9,
                16 => raw % 17,
                _ => raw,
            }
        } else {
            raw % u64::from(bits)
        };

        let msb = 1u64 << (bits - 1);
        let cf_before = self.cpu.flags.contains(RFlags::CF);
        let mut cf = cf_before;
        let mut v = a;
        // Un cran à la fois : c'est lent, et c'est mot pour mot ce que
        // dit le manuel — ce qui compte davantage ici.
        for _ in 0..count {
            match kind {
                RotateKind::Left => {
                    let top = v & msb != 0;
                    v = ((v << 1) & mask) | u64::from(top);
                    cf = top;
                }
                RotateKind::Right => {
                    let bottom = v & 1;
                    v = (v >> 1) | (bottom << (bits - 1));
                    cf = bottom == 1;
                }
                RotateKind::CarryLeft => {
                    let top = v & msb != 0;
                    v = ((v << 1) & mask) | u64::from(cf);
                    cf = top;
                }
                RotateKind::CarryRight => {
                    let bottom = v & 1 == 1;
                    v = (v >> 1) | (u64::from(cf) << (bits - 1));
                    cf = bottom;
                }
            }
        }
        // `rol al, 8` ne change rien à AL mais renseigne tout de même la
        // retenue : elle vaut le bit qui serait sorti.
        match kind {
            RotateKind::Left => cf = v & 1 == 1,
            RotateKind::Right => cf = v & msb != 0,
            _ => {}
        }

        let mut flags = self.cpu.flags;
        flags.set(RFlags::CF, cf);
        if raw == 1 {
            let of = match kind {
                RotateKind::Left | RotateKind::CarryLeft => (v & msb != 0) != cf,
                RotateKind::Right => (v & msb != 0) != (v & (msb >> 1) != 0),
                RotateKind::CarryRight => (a & msb != 0) != cf_before,
            };
            flags.set(RFlags::OF, of);
        }
        self.cpu.flags = flags;
        self.write_op(instr, 0, v)?;
        Ok(())
    }

    /// `shld`/`shrd` : la destination se décale, et les bits qui entrent
    /// viennent de la source au lieu d'être des zéros. C'est ainsi qu'on
    /// décale une valeur plus large qu'un registre.
    fn double_shift(&mut self, instr: &Instruction, left: bool) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let bits = w.bits();
        let mask = w.mask();
        let dest = self.read_op(instr, 0)? & mask;
        let src = self.read_op(instr, 1)? & mask;
        let count = (self.read_op(instr, 2)? & if bits == 64 { 0x3f } else { 0x1f }) as u32;
        if count == 0 {
            return Ok(());
        }
        if count >= bits {
            // Intel laisse le résultat indéfini ; ne rien faire est la
            // seule réponse qui ne raconte pas d'histoire.
            return Ok(());
        }

        let (result, cf) = if left {
            (
                ((dest << count) | (src >> (bits - count))) & mask,
                (dest >> (bits - count)) & 1 == 1,
            )
        } else {
            (
                ((dest >> count) | (src << (bits - count))) & mask,
                (dest >> (count - 1)) & 1 == 1,
            )
        };

        let mut flags = logic_flags(result, w);
        flags.set(RFlags::CF, cf);
        if count == 1 {
            // Le signe a-t-il changé ? C'est tout ce que dit OF ici.
            flags.set(RFlags::OF, (dest ^ result) & (1 << (bits - 1)) != 0);
        }
        self.cpu.flags.apply_arith(flags);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    /// `bt`, `bts`, `btr`, `btc` : le bit désigné part dans CF, et
    /// l'instruction le laisse, le pose, l'efface ou le retourne.
    fn bit_test(&mut self, instr: &Instruction, op: BitOp) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let raw = self.read_op(instr, 1)?;

        // Décalage dans un registre et destination en mémoire : le bit
        // désigné n'est pas borné par l'opérande, il compte dans une
        // suite d'octets. `bt [drapeaux], rax` avec RAX = 100 va donc
        // chercher le treizième octet — ce qui permet de tester un
        // tableau de bits sans calculer soi-même l'octet.
        if instr.op_kind(0) == OpKind::Memory && instr.op_kind(1) == OpKind::Register {
            let base = self.effective_address(instr, 0)?;
            let offset = sign_extend(raw, self.src_width(instr, 1)?) as i64;
            let addr = base.wrapping_add_signed(offset.div_euclid(8));
            let bit = offset.rem_euclid(8) as u32;
            let byte = u64::from(self.cpu.mem.read_u8(addr)?);
            self.cpu.flags.set(RFlags::CF, (byte >> bit) & 1 == 1);
            if let Some(new) = op.apply(byte, bit) {
                self.record_memory(addr, 1);
                self.cpu.mem.write_u8(addr, new as u8)?;
            }
            return Ok(());
        }

        let value = self.read_op(instr, 0)? & w.mask();
        let bit = (raw & u64::from(w.bits() - 1)) as u32;
        self.cpu.flags.set(RFlags::CF, (value >> bit) & 1 == 1);
        if let Some(new) = op.apply(value, bit) {
            self.write_op(instr, 0, new)?;
        }
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
            | OpKind::Immediate32to64
            // Le second immédiat d'`enter` : la seule instruction du jeu
            // qui en porte deux.
            | OpKind::Immediate8_2nd => Ok(instr.immediate(op_index)),
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
            heap: self.heap,
            ticks: self.ticks,
            breaches: self.breaches.len(),
        });
    }

    /// Note les octets sur le point d'être écrasés. Lus *avant*
    /// l'écriture, évidemment — c'est toute l'idée.
    fn record_memory(&mut self, addr: u64, len: usize) {
        if self.history.is_empty() {
            return;
        }
        let Ok(old) = self.cpu.mem.peek_bytes(addr, len) else {
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
            // Défaire, c'est remettre ce qui était là : les droits ne
            // s'appliquent qu'au programme, pas au journal qui le
            // rembobine.
            let _ = self.cpu.mem.poke(*addr, bytes);
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
        self.heap = undo.heap;
        self.map_heap();
        self.ticks = undo.ticks;
        self.breaches.truncate(undo.breaches);
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
        self.rearm_registers();
        true
    }

    /// Les appels en cours, du plus ancien au plus récent.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Ce que les sous-routines déjà revenues n'ont pas respecté.
    pub fn breaches(&self) -> &[AbiBreach] {
        &self.breaches
    }

    /// Au retour d'une sous-routine : a-t-elle rendu ce qu'elle devait ?
    ///
    /// Les registres confiés par la convention, et la pile — un `push`
    /// sans `pop` ne se voit pas autrement qu'ici, et c'est la faute la
    /// plus coûteuse à chercher soi-même.
    fn check_convention(&mut self, frame: &Frame) {
        for (index, (name, reg)) in PRESERVED.iter().enumerate() {
            if self.cpu.regs.get64(*reg) != frame.preserved[index] {
                self.note_breach(frame, name);
            }
        }
        // À cet instant, RSP a déjà dépilé l'adresse de retour : il doit
        // valoir ce qu'il valait juste avant l'appel.
        if self.cpu.regs.get64(Reg64::Rsp) != frame.rsp.wrapping_add(8) {
            self.note_breach(frame, "RSP");
        }
    }

    fn note_breach(&mut self, frame: &Frame, register: &'static str) {
        let breach = AbiBreach {
            call_site: frame.call_site,
            entry: frame.entry,
            register,
        };
        // Une boucle qui appelle mille fois la même routine fautive n'a
        // rien à dire de plus la millième fois.
        if !self.breaches.contains(&breach) {
            self.breaches.push(breach);
        }
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

    /// Remplace les registres surveillés, et les arme sur l'état courant :
    /// surveiller un registre, c'est dire « préviens-moi s'il change **à
    /// partir de maintenant** ».
    pub fn set_watched_registers(&mut self, registers: &[(&'static str, Reg64)]) {
        self.watched_registers = registers
            .iter()
            .map(|&(name, reg)| (name, reg, self.cpu.regs.get64(reg)))
            .collect();
    }

    /// Les registres surveillés, avec leur valeur au dernier armement.
    pub fn watched_registers(&self) -> &[(&'static str, Reg64, u64)] {
        &self.watched_registers
    }

    /// Le premier registre surveillé qui a changé depuis le dernier
    /// passage. Réarme au passage, comme pour la mémoire.
    fn changed_register(&mut self) -> Option<(&'static str, u64, u64)> {
        for index in 0..self.watched_registers.len() {
            let (name, reg, old) = self.watched_registers[index];
            let new = self.cpu.regs.get64(reg);
            if new != old {
                self.watched_registers[index].2 = new;
                return Some((name, old, new));
            }
        }
        None
    }

    /// Réarme les registres surveillés sur l'état courant, sans rien
    /// signaler — après un retour en arrière, c'est d'ici qu'on repart.
    fn rearm_registers(&mut self) {
        for entry in &mut self.watched_registers {
            entry.2 = self.cpu.regs.get64(entry.1);
        }
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
        register_value(&self.cpu.regs, reg).ok_or(EmuError::UnsupportedRegister {
            backend: Self::NAME,
            register: format!("{reg:?}"),
        })
    }

    fn set_reg(&mut self, reg: Register, value: u64) -> Result<(), EmuError> {
        let (r, w) = map_register(reg)?;
        if is_high_byte(reg) {
            self.cpu.regs.set8h(r, value as u8);
            return Ok(());
        }
        match w {
            Width::W64 => self.cpu.regs.set64(r, value),
            Width::W32 => self.cpu.regs.set32(r, value as u32),
            Width::W16 => self.cpu.regs.set16(r, value as u16),
            Width::W8 => self.cpu.regs.set8l(r, value as u8),
        }
        Ok(())
    }

    fn effective_address(&self, instr: &Instruction, op_index: u32) -> Result<u64, EmuError> {
        effective_address(&self.cpu.regs, instr, op_index)
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

/// La valeur d'un registre entier, lue sur sa propre largeur : `eax` rend
/// quatre octets, `ah` l'octet haut de `rax`.
///
/// Libre, comme [`effective_address`] : l'interface montre ces valeurs à
/// côté de l'instruction qui va les lire, sans tenir de moteur. Rend
/// `None` pour ce qui n'est pas un registre entier — vectoriels,
/// segments, registres de contrôle.
pub fn register_value(regs: &Registers, reg: Register) -> Option<u64> {
    let (r, w) = map_register(reg).ok()?;
    if is_high_byte(reg) {
        return Some(u64::from(regs.get8h(r)));
    }
    Some(match w {
        Width::W64 => regs.get64(r),
        Width::W32 => regs.get32(r) as u64,
        Width::W16 => regs.get16(r) as u64,
        Width::W8 => regs.get8l(r) as u64,
    })
}

/// La condition d'un `jcc`, `setcc` ou `cmovcc` est-elle vraie avec ces
/// indicateurs ?
///
/// Libre, comme [`effective_address`] : l'interface s'en sert pour dire,
/// *avant* d'exécuter, si le saut sera pris — « quel saut après quelle
/// comparaison » étant l'endroit précis où l'on se trompe en apprenant.
///
/// Rend `None` pour une instruction sans condition.
pub fn condition_met(code: ConditionCode, flags: RFlags) -> Option<bool> {
    let zf = flags.contains(RFlags::ZF);
    let sf = flags.contains(RFlags::SF);
    let cf = flags.contains(RFlags::CF);
    let of = flags.contains(RFlags::OF);
    let pf = flags.contains(RFlags::PF);
    Some(match code {
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
        ConditionCode::None => return None,
    })
}

/// Les indicateurs que cette condition consulte, et ce que la condition
/// en fait — écrit comme on l'énonce : `ZF = 1`, `SF = OF`.
///
/// C'est la réponse à « pourquoi ce saut est-il pris ? », et elle ne
/// s'invente pas : `jg` regarde trois indicateurs, `ja` deux, et ce ne
/// sont pas les mêmes selon qu'on compare des nombres signés ou non.
pub fn condition_rule(code: ConditionCode) -> Option<&'static str> {
    Some(match code {
        ConditionCode::e => "ZF = 1",
        ConditionCode::ne => "ZF = 0",
        ConditionCode::b => "CF = 1",
        ConditionCode::ae => "CF = 0",
        ConditionCode::be => "CF = 1 ou ZF = 1",
        ConditionCode::a => "CF = 0 et ZF = 0",
        ConditionCode::l => "SF ≠ OF",
        ConditionCode::ge => "SF = OF",
        ConditionCode::le => "ZF = 1 ou SF ≠ OF",
        ConditionCode::g => "ZF = 0 et SF = OF",
        ConditionCode::s => "SF = 1",
        ConditionCode::ns => "SF = 0",
        ConditionCode::o => "OF = 1",
        ConditionCode::no => "OF = 0",
        ConditionCode::p => "PF = 1",
        ConditionCode::np => "PF = 0",
        ConditionCode::None => return None,
    })
}

/// La comparaison lit-elle ses opérandes comme signés ?
///
/// `ja` et `jg` disent tous les deux « plus grand », et ne donnent pas
/// la même réponse sur les mêmes octets : c'est l'erreur classique, et
/// elle mérite d'être dite à l'endroit où on la fait.
pub fn condition_signed(code: ConditionCode) -> Option<bool> {
    Some(match code {
        ConditionCode::l | ConditionCode::ge | ConditionCode::le | ConditionCode::g => true,
        ConditionCode::b | ConditionCode::ae | ConditionCode::be | ConditionCode::a => false,
        _ => return None,
    })
}

/// L'adresse que désigne l'opérande mémoire `op_index`, telle que le
/// processeur la calculerait avec les registres *actuels*.
///
/// Libre, et non méthode : l'interface s'en sert pour montrer où pointe
/// `[rsi + rcx*4 + 8]` **avant** d'exécuter l'instruction, sans avoir à
/// tenir un moteur complet.
///
/// Rend `None` quand l'adresse dépend de FS ou GS — namsdec ne simule
/// pas la base que le noyau y installe, et une adresse fausse serait
/// pire que pas d'adresse du tout.
pub fn effective_address(regs: &Registers, instr: &Instruction, op_index: u32) -> Option<u64> {
    let next_ip = instr.next_ip();
    instr.virtual_address(op_index, 0, |register, _idx, _size| match register {
        Register::None => Some(0),
        Register::RIP | Register::EIP => Some(next_ip),
        // iced réclame aussi le registre de segment : une adresse
        // virtuelle, c'est la base du segment plus le décalage. En 64
        // bits, CS/DS/ES/SS ont une base nulle — c'est le modèle plat,
        // et c'est ce que Linux donne à un programme. Sans ce cas,
        // *aucun* opérande mémoire ne se calculait : `mov [case], rax`
        // échouait avant même de lire la mémoire.
        Register::CS | Register::DS | Register::ES | Register::SS => Some(0),
        // FS et GS, eux, ont une base que le noyau installe (TLS,
        // canari de pile). namsdec ne la simule pas.
        Register::FS | Register::GS => None,
        other => map_register(other).ok().map(|(r, _)| regs.get64(r)),
    })
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
    // Le droit d'*exécuter*, et non celui de lire : c'est ce qui
    // distingue « sauter dans ses données » de « lire ses données ». Un
    // seul octet suffit à le dire — l'instruction peut déborder sur la
    // zone suivante, et c'est le premier qui compte.
    mem.check(addr, 1, Access::Execute)?;
    let avail = (range.end - addr).min(16) as usize;
    // `peek_bytes` : les octets d'une instruction se lisent même dans une
    // zone qui n'accorde pas la lecture. Un processeur exécute du code
    // « exécutable seulement » sans broncher.
    mem.peek_bytes(addr, avail)
        .map_err(|_| EmuError::InvalidInstruction { addr })
}

#[cfg(test)]
mod tests;
