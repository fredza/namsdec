//! Interpréteur x86-64 "maison" : le décodage octets -> instruction
//! structurée est délégué à `namsdec-disasm` (donc à `iced-x86`), mais toute
//! la sémantique d'exécution (effets sur registres/flags/mémoire) est
//! écrite ici, à la main, pour rester transparente et pédagogique.

mod libc;
pub use libc::name_reads_input;
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

pub use libc::{PROVIDED as PROVIDED_FUNCTIONS, provides};

pub mod explain;
pub mod telescope;

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
    /// Le programme appelle une fonction qui vivrait dans une
    /// bibliothèque. namsdec en fournit une poignée, écrites en Rust ;
    /// celle-ci n'en fait pas partie.
    #[error("{backend}: unsupported library function: {name}")]
    UnsupportedLibrary { backend: &'static str, name: String },
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
    /// L'adresse d'un `int3` rencontré, s'il y en a eu un.
    ///
    /// `hlt` et `int3` arrêtent tous deux la machine, et ne veulent pas
    /// dire la même chose : le premier est une fin, le second un point
    /// d'arrêt que le programme se pose à lui-même. Les confondre, c'est
    /// annoncer « terminé » à quelqu'un dont le programme attend qu'on le
    /// relance.
    software_breakpoint: Option<u64>,
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
    /// L'arène d'avant : un `mmap` aussi.
    mmap: Mmap,
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
///
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

    /// Le pas qui a écrit dans ce registre pour la dernière fois, par
    /// son rang dans [`Backend::trace`].
    ///
    /// « D'où vient cette valeur ? » est la question qu'on pose devant
    /// un registre qui ne contient pas ce qu'on croyait, et le chercher
    /// à la main veut dire relire son programme à l'envers. L'historique
    /// le sait : il garde les registres d'avant chaque pas.
    fn last_write_to(&self, reg: Reg64) -> Option<usize>;

    /// Le pas qui a écrit ce registre pour la dernière fois **avant** un
    /// pas donné.
    ///
    /// C'est ce qui permet de remonter une chaîne : RAX vient de ce pas,
    /// qui lisait RCX, lequel vient de ce pas-là. Sans la borne, on
    /// retrouverait chaque fois la même écriture, la plus récente, et la
    /// chaîne tournerait en rond.
    ///
    /// `avant` est exclusif : la longueur de l'historique désigne
    /// « maintenant », et rend alors la même réponse que
    /// `last_write_to`.
    fn last_write_to_before(&self, reg: Reg64, avant: usize) -> Option<usize>;

    /// Le pas qui a écrit à cette adresse pour la dernière fois.
    ///
    /// La même question que pour un registre, posée à la mémoire : « qui
    /// a mis ça là ? ». Elle est plus difficile encore à la main, parce
    /// qu'une écriture mémoire ne laisse aucune trace visible dans le
    /// code — il faut relire chaque instruction en se demandant où elle
    /// pointait.
    fn last_write_to_memory(&self, addr: u64) -> Option<usize>;

    /// Le chemin parcouru : l'adresse de chaque pas qu'on peut encore
    /// défaire, du plus ancien au plus récent.
    ///
    /// Le compteur de passages dit *combien de fois* une ligne a tourné ;
    /// celle-ci dit *dans quel ordre* le programme est passé — ce qui
    /// répond à « comment suis-je arrivé là ? », la question qu'on pose
    /// devant un état qu'on n'attendait pas.
    fn trace(&self) -> &VecDeque<Step>;

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

    /// L'adresse du `int3` sur lequel la machine s'est arrêtée, s'il y en
    /// a eu un. `None` quand l'arrêt vient d'ailleurs.
    fn software_breakpoint(&self) -> Option<u64>;
}

/// Ce qu'un pas a fait, au-delà d'avancer.
///
/// Une chronologie d'adresses se lit mal : cent lignes se ressemblent
/// toutes. Ce qu'on y cherche, ce sont les moments où le programme a
/// changé de direction ou touché autre chose que ses registres — et ce
/// sont eux qu'on veut voir sans les déchiffrer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// Rien de particulier : un calcul, une copie de registre.
    Plain,
    /// Un appel : on est entré dans une sous-routine.
    Call,
    /// Un retour : on en est sorti.
    Return,
    /// Un saut effectivement pris — un `jcc` qui ne saute pas est un pas
    /// ordinaire, et c'est bien la distinction qui compte.
    Jump,
    /// Un appel système.
    Syscall,
    /// Le pas a écrit en mémoire.
    Write,
}

/// Un pas du chemin parcouru.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// L'adresse de l'instruction exécutée.
    pub addr: u64,
    pub kind: StepKind,
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

/// L'arène de `mmap` : la mémoire que le noyau donne **en dehors** du
/// tas, et par laquelle passe tout allocateur un peu sérieux.
///
/// `brk` sert à agrandir le tas d'un seul tenant ; `mmap` sert à obtenir
/// un bloc indépendant, qu'on rendra sans toucher au reste. La glibc s'en
/// sert pour les grosses allocations, pour ses tampons d'entrée-sortie et
/// pour la mémoire locale des fils. Un programme lié statiquement n'en
/// fait pas trois cents pas sans.
///
/// Ici, pas de pagination ni de recherche de place : un curseur qui
/// monte. Rendre la mémoire (`munmap`) ne la reprend donc pas — ce qui
/// est dit dans le code plutôt que caché.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mmap {
    start: u64,
    current: u64,
    limit: u64,
}

impl Mmap {
    /// Combien d'octets ont été donnés.
    pub fn len(&self) -> u64 {
        self.current - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.current == self.start
    }

    /// Le plus haut que l'arène puisse monter.
    pub fn limit(&self) -> u64 {
        self.limit
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
    /// L'adresse d'un `int3` rencontré, s'il y en a eu un.
    ///
    /// `hlt` et `int3` arrêtent tous deux la machine, et ne veulent pas
    /// dire la même chose : le premier est une fin, le second un point
    /// d'arrêt que le programme se pose à lui-même. Les confondre, c'est
    /// annoncer « terminé » à quelqu'un dont le programme attend au
    /// contraire qu'on le relance.
    software_breakpoint: Option<u64>,
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
    /// L'arène de `mmap`, telle que les allocations l'ont laissée.
    mmap: Mmap,
    /// Les fonctions de bibliothèque, par l'adresse de leur borne.
    ///
    /// Le chargeur pose une borne à la place de chaque fonction qu'un
    /// binaire emprunte ; arriver dessus n'est pas « sauter dans des
    /// données », c'est appeler `printf`. namsdec en fournit une poignée,
    /// écrites en Rust ; les autres s'arrêtent en se nommant.
    externals: BTreeMap<u64, String>,
    /// Là où `main` revient : une borne à part, qui transforme son
    /// résultat en code de sortie.
    main_return: u64,
    /// La case d'`errno`, allouée au premier `__errno_location`.
    errno_at: u64,
    /// Les entorses à la convention d'appel relevées jusqu'ici.
    breaches: Vec<AbiBreach>,
    /// Combien d'instructions ont été exécutées depuis le chargement.
    /// C'est ce que rend `rdtsc` : le temps qu'on peut mesurer ici.
    ticks: u64,
    /// Combien de fois chaque adresse a été exécutée.
    visits: BTreeMap<u64, u64>,
    /// Le chemin parcouru, pas par pas — aussi loin que l'on peut
    /// revenir en arrière, et pas plus : une chronologie qui montrerait
    /// des pas qu'on ne peut plus défaire ne serait pas une chronologie,
    /// seulement une liste.
    trace: VecDeque<Step>,
}

impl Backend for Emulator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn executed(&self) -> u64 {
        self.ticks
    }

    fn trace(&self) -> &VecDeque<Step> {
        &self.trace
    }

    fn last_write_to_memory(&self, addr: u64) -> Option<usize> {
        // Le journal garde, pour chaque pas, les octets d'avant et leur
        // adresse : celui qui couvre l'adresse cherchée est celui qui a
        // écrit dessus.
        self.history
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, undo)| {
                undo.memory
                    .iter()
                    .any(|(start, bytes)| (*start..*start + bytes.len() as u64).contains(&addr))
                    .then_some(index)
            })
    }

    fn last_write_to(&self, reg: Reg64) -> Option<usize> {
        self.last_write_to_before(reg, self.history.len())
    }

    fn last_write_to_before(&self, reg: Reg64, avant: usize) -> Option<usize> {
        // On remonte le journal en gardant la valeur « d'après » : le
        // premier pas dont la valeur d'avant diffère est celui qui a
        // écrit. Les autres ne l'ont pas touché, quoi qu'ils aient fait
        // par ailleurs.
        let avant = avant.min(self.history.len());
        // La valeur du registre juste avant le pas `avant` : celle que
        // ce pas a lue. Au bout de l'historique, c'est celle de
        // maintenant.
        let mut après = match self.history.get(avant) {
            Some(undo) => undo.regs.get64(reg),
            None => self.cpu.regs.get64(reg),
        };
        for (index, undo) in self.history.iter().enumerate().take(avant).rev() {
            let valeur = undo.regs.get64(reg);
            if valeur != après {
                return Some(index);
            }
            après = valeur;
        }
        None
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

    fn software_breakpoint(&self) -> Option<u64> {
        self.software_breakpoint
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
            software_breakpoint: None,
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
            // Sans arène déclarée, `mmap` n'a rien à donner et le dit.
            mmap: Mmap {
                start: end,
                current: end,
                limit: end,
            },
            externals: BTreeMap::new(),
            main_return: 0,
            errno_at: 0,
            ticks: 0,
            visits: BTreeMap::new(),
            trace: VecDeque::new(),
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

    /// Installe l'arène de `mmap`.
    ///
    /// Comme pour le tas, c'est l'appelant qui la place : lui seul sait
    /// ce qu'il a chargé et où la pile commence. Sans arène, `mmap`
    /// répond « plus de mémoire » — ce qui est la vérité.
    pub fn set_mmap(&mut self, start: u64, limit: u64) {
        let range = self.cpu.mem.range();
        let start = page_up(start).clamp(range.start, range.end);
        let limit = limit.clamp(start, range.end);
        self.mmap = Mmap {
            start,
            current: start,
            limit,
        };
    }

    /// L'arène de `mmap`, telle que les allocations l'ont laissée.
    pub fn mmap(&self) -> Mmap {
        self.mmap
    }

    /// Installe les bornes des fonctions de bibliothèque : adresse -> nom.
    ///
    /// `main_return` est celle sur laquelle `main` revient — une borne à
    /// part, que `__libc_start_main` empile avant de sauter.
    pub fn set_externals(&mut self, table: BTreeMap<u64, String>, main_return: u64) {
        self.externals = table;
        self.main_return = main_return;
    }

    /// Le nom de la fonction dont la borne est à cette adresse.
    pub fn external_at(&self, addr: u64) -> Option<&str> {
        self.externals.get(&addr).map(String::as_str)
    }

    /// L'adresse sur laquelle `main` revient.
    fn main_return_addr(&self) -> u64 {
        self.main_return
    }

    /// Donne `size` octets de tas à la bibliothèque fournie.
    ///
    /// Un allocateur minimal : un curseur qui monte dans le tas, et huit
    /// octets de garde devant chaque bloc pour y noter sa taille — comme
    /// le fait un vrai `malloc`, et pour la même raison : `free` et
    /// `realloc` doivent pouvoir la retrouver.
    ///
    /// La mémoire rendue par `free` n'est pas reprise. C'est une
    /// simplification, et elle est dite : un programme qui relit ce qu'il
    /// a libéré retrouve ses données, ce qu'un vrai `malloc` fait aussi
    /// souvent — et c'est bien pourquoi ce bogue-là est si difficile à
    /// voir.
    fn libc_alloc(&mut self, size: u64) -> Option<u64> {
        /// L'alignement que tout `malloc` promet : de quoi y poser
        /// n'importe quel type, `long double` compris.
        const ALIGN: u64 = 16;
        let base = (self.heap.current + 8).div_ceil(ALIGN) * ALIGN;
        let fin = base.checked_add(size.max(1))?;
        if self.set_break(fin) < fin {
            return None;
        }
        self.record_memory(base - 8, 8);
        self.cpu.mem.poke(base - 8, &size.to_le_bytes()).ok()?;
        Some(base)
    }

    /// La taille notée devant un bloc rendu par la bibliothèque fournie.
    fn block_size(&self, addr: u64) -> u64 {
        if addr < self.heap.start + 8 {
            return 0;
        }
        self.cpu.mem.peek_u64(addr - 8).unwrap_or(0)
    }

    /// La case d'`errno`, allouée au premier usage.
    fn errno_slot(&mut self) -> u64 {
        if self.errno_at == 0 {
            self.errno_at = self.libc_alloc(8).unwrap_or(0);
        }
        self.errno_at
    }

    /// Un nombre pseudo-aléatoire, de la même suite reproductible que
    /// `getrandom` : le même programme doit donner deux fois le même
    /// résultat, sans quoi revenir en arrière n'aurait plus de sens.
    fn pseudo_random_u32(&self) -> u32 {
        let mut state = 0x9e37_79b9_7f4a_7c15u64 ^ self.ticks.wrapping_mul(0x2545_f491_4f6c_dd1d);
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // `rand` rend un entier positif, borné par RAND_MAX.
        (state >> 33) as u32 & 0x7fff_ffff
    }

    /// Donne `len` octets d'arène, projetés avec les droits demandés.
    ///
    /// Rend `None` quand il n'y a plus de place — ce que le noyau
    /// signale par `-ENOMEM`, et qu'un allocateur sait encaisser.
    fn mmap_anonymous(&mut self, len: u64, perms: namsdec_core::Perms) -> Option<u64> {
        let len = page_up(len.max(1));
        let start = self.mmap.current;
        let fin = start.checked_add(len)?;
        if fin > self.mmap.limit {
            return None;
        }
        self.mmap.current = fin;
        // Une page rendue par `mmap` est **vierge** : le noyau ne donne
        // jamais les données de quelqu'un d'autre. Ici, la mémoire a déjà
        // servi si un `munmap` est passé avant, d'où la remise à zéro.
        let _ = self.cpu.mem.poke(start, &vec![0u8; len as usize]);
        // Seulement si la mémoire est déjà découpée en zones : en
        // déclarer une dans un bloc qui n'en a aucune rendrait tout le
        // reste — le programme lui-même — inadressable.
        if self.cpu.mem.has_regions() {
            self.cpu.mem.map(start..fin, perms, "mmap");
        }
        Some(start)
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

    /// Décrit un descripteur ouvert comme le ferait `fstat`.
    ///
    /// Une bibliothèque C interroge la sortie standard avant d'écrire son
    /// premier caractère : selon la réponse, elle tamponne par ligne (un
    /// terminal) ou par blocs de quelques kilo-octets (un fichier). Sans
    /// réponse, elle n'écrit rien du tout — et l'on cherche longtemps
    /// pourquoi un `printf` ne sort pas.
    ///
    /// namsdec répond « un terminal » : la sortie apparaît alors ligne à
    /// ligne, au moment où le programme l'écrit, ce qui est exactement ce
    /// qu'on veut en avançant pas à pas.
    fn fstat(&mut self, fd: u64, buf: u64) -> Result<i64, EmuError> {
        /// `struct stat` d'x86-64 : 144 octets, dont seuls quelques
        /// champs intéressent qui que ce soit ici.
        const STAT_SIZE: usize = 144;
        /// Un périphérique caractère, lisible et inscriptible par son
        /// propriétaire : `S_IFCHR | 0620`, ce qu'est un terminal.
        const S_IFCHR: u32 = 0x2000;
        const MODE: u32 = S_IFCHR | 0o620;
        if fd > 2 {
            return Ok(-9);
        }
        if buf == 0 {
            return Ok(-14);
        }
        let mut stat = [0u8; STAT_SIZE];
        stat[24..28].copy_from_slice(&MODE.to_le_bytes());
        stat[16..24].copy_from_slice(&1u64.to_le_bytes()); // st_nlink
        stat[56..64].copy_from_slice(&1024i64.to_le_bytes()); // st_blksize
        self.cpu.mem.poke(buf, &stat)?;
        Ok(0)
    }

    /// Remplit la `struct utsname` de `uname`.
    fn uname(&mut self, buf: u64) -> Result<(), EmuError> {
        /// Six champs de 65 octets, bout à bout.
        const CHAMP: usize = 65;
        let champs = [
            "Linux",
            "namsdec",
            "6.0.0-namsdec",
            "#1 SMP namsdec",
            "x86_64",
            "(none)",
        ];
        let mut bloc = vec![0u8; CHAMP * champs.len()];
        for (index, texte) in champs.iter().enumerate() {
            let octets = texte.as_bytes();
            bloc[index * CHAMP..index * CHAMP + octets.len()].copy_from_slice(octets);
        }
        self.cpu.mem.poke(buf, &bloc)?;
        Ok(())
    }

    /// `writev` et `readv` : la même chose que `write` et `read`, mais sur
    /// une liste de morceaux.
    ///
    /// Une bibliothèque C s'en sert pour vider un tampon et ajouter la
    /// suite en un seul appel — ce qui explique qu'un `printf` puisse ne
    /// jamais passer par `write`.
    fn vector_io(&mut self, fd: u64, iov: u64, count: u64, lecture: bool) -> Result<i64, EmuError> {
        /// Un `struct iovec` : un pointeur, une longueur.
        const IOVEC: u64 = 16;
        /// Le noyau en refuse plus de mille vingt-quatre.
        const IOV_MAX: u64 = 1024;
        if count > IOV_MAX {
            return Ok(-22);
        }
        let mut total = 0i64;
        for index in 0..count {
            let at = iov + index * IOVEC;
            let base = self.cpu.mem.read_u64(at)?;
            let len = self.cpu.mem.read_u64(at + 8)?;
            if len == 0 {
                continue;
            }
            let fait = if lecture {
                self.read(fd, base, len)?
            } else {
                self.write(fd, base, len)?
            };
            if fait < 0 {
                return Ok(if total > 0 { total } else { fait });
            }
            total += fait;
            // Une lecture plus courte que demandé met fin à l'appel :
            // c'est ainsi que se termine un fichier.
            if (fait as u64) < len {
                break;
            }
        }
        Ok(total)
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
        /// Les quelques appels qui *renseignent* sans rien changer au
        /// monde. Un programme qui les fait n'attend pas grand-chose,
        /// mais s'arrêter dessus l'empêcherait d'aller plus loin.
        const SYS_SCHED_YIELD: u64 = 24;
        const SYS_NANOSLEEP: u64 = 35;
        const SYS_GETPID: u64 = 39;
        const SYS_GETUID: u64 = 102;
        const SYS_GETEUID: u64 = 107;
        /// `getrandom` remplit un tampon. Ce que namsdec y met n'est pas
        /// du hasard : c'est une suite reproductible, sans quoi le même
        /// programme ne donnerait pas deux fois le même résultat et le
        /// retour en arrière n'aurait plus de sens.
        const SYS_GETRANDOM: u64 = 318;
        /// `arch_prctl` : le seul moyen, en espace utilisateur, de
        /// déplacer la base FS. Un binaire lié statiquement s'en sert
        /// pour installer sa propre mémoire locale au fil, avant même
        /// d'avoir appelé quoi que ce soit.
        const SYS_ARCH_PRCTL: u64 = 158;
        const ARCH_SET_GS: u64 = 0x1001;
        const ARCH_SET_FS: u64 = 0x1002;
        const ARCH_GET_FS: u64 = 0x1003;
        const ARCH_GET_GS: u64 = 0x1004;
        /// Les appels par lesquels une bibliothèque C annonce au noyau de
        /// quoi gérer les fils d'exécution. Sans fils, il n'y a rien à
        /// annoncer — mais refuser l'appel arrêterait le programme au
        /// milieu de son démarrage, avant sa première ligne utile.
        /// `prctl` règle mille détails d'un processus — son nom, son
        /// droit de vidanger, sa graine de hasard. Une bibliothèque C
        /// liée statiquement en fait un au démarrage et continue quoi
        /// qu'on réponde : refuser l'arrêtait avant son premier pas.
        const SYS_PRCTL: u64 = 157;
        /// `ptrace` demande si quelqu'un regarde. C'est l'anti-débogage
        /// le plus répandu : le programme s'attache lui-même, et s'il
        /// échoue, c'est qu'un débogueur est déjà là.
        ///
        /// namsdec répond zéro — « personne ne te regarde ». C'est un
        /// mensonge, et c'est délibéré : l'outil est fait pour observer
        /// des programmes qui préféreraient ne pas l'être, et le dire
        /// vaut mieux que de s'arrêter sur la question.
        const SYS_PTRACE: u64 = 101;
        /// `fork` et ses deux cousins : `clone` quand on lui demande un
        /// processus (pile nulle, `SIGCHLD`), et `vfork`.
        ///
        /// Ils forment la seconde moitié de l'anti-débogage par `ptrace`,
        /// et la plus retorse : le programme se dédouble, le fils
        /// s'attache par `PTRACE_TRACEME`, le père l'attend. Sous un
        /// débogueur, l'attache échoue et le fils se saborde ; le père lit
        /// son code de sortie et se saborde à son tour. Le programme ne
        /// montre jamais rien.
        const SYS_CLONE: u64 = 56;
        const SYS_FORK: u64 = 57;
        const SYS_VFORK: u64 = 58;
        /// Les trois façons d'envoyer un signal à quelqu'un — ici, à
        /// soi-même : `kill(pid)`, `tkill(tid)`, `tgkill(tgid, tid)`.
        ///
        /// C'est le troisième temps du montage : après s'être dédoublé et
        /// attaché, le fils s'arrête net par `SIGSTOP` pour dire au père
        /// « je suis prêt », et attend d'être relancé.
        const SYS_KILL: u64 = 62;
        const SYS_TKILL: u64 = 200;
        const SYS_TGKILL: u64 = 234;
        /// `SIGSTOP` met en pause, `SIGCONT` relance. Les seuls que l'on
        /// sache encaisser : ils ne demandent rien d'autre que de laisser
        /// le programme courir.
        const SIGCONT: u64 = 18;
        const SIGSTOP: u64 = 19;
        const SYS_SET_TID_ADDRESS: u64 = 218;
        const SYS_SET_ROBUST_LIST: u64 = 273;
        const SYS_RSEQ: u64 = 334;
        /// `mmap` et ses voisins : la mémoire qu'on demande en dehors du
        /// tas. Toute bibliothèque C sérieuse passe par là — pour ses
        /// tampons d'entrée-sortie, pour les grosses allocations, pour la
        /// mémoire locale d'un fil.
        const SYS_MMAP: u64 = 9;
        const SYS_MPROTECT: u64 = 10;
        const SYS_MUNMAP: u64 = 11;
        /// `MAP_ANONYMOUS` : de la mémoire vierge, sans fichier derrière.
        /// C'est le seul `mmap` que namsdec sait rendre — il n'y a pas de
        /// système de fichiers ici.
        const MAP_ANONYMOUS: u64 = 0x20;
        const PROT_READ: u64 = 1;
        const PROT_WRITE: u64 = 2;
        const PROT_EXEC: u64 = 4;
        /// `fstat` sur la sortie standard : une bibliothèque C le fait
        /// avant d'écrire son premier caractère, pour choisir comment
        /// tamponner. Sans réponse, elle n'écrit rien du tout.
        const SYS_FSTAT: u64 = 5;
        const SYS_NEWFSTATAT: u64 = 262;
        /// `ioctl` : la même question, posée autrement — « es-tu un
        /// terminal ? »
        const SYS_IOCTL: u64 = 16;
        /// `writev` écrit plusieurs morceaux d'un coup. `printf` s'en
        /// sert dès qu'il doit vider un tampon et ajouter du texte.
        const SYS_WRITEV: u64 = 20;
        const SYS_READV: u64 = 19;
        /// Les limites du processus. La glibc lit celle de la pile au
        /// démarrage, et s'arrête si personne ne répond.
        const SYS_PRLIMIT64: u64 = 302;
        const SYS_GETRLIMIT: u64 = 97;
        const RLIMIT_STACK: u64 = 3;
        /// `uname` : qui es-tu, noyau ?
        const SYS_UNAME: u64 = 63;
        /// Les liens symboliques : un programme y lit son propre chemin.
        /// Il n'y a pas de système de fichiers, et le dire (`-EINVAL`)
        /// vaut mieux que de s'arrêter.
        const SYS_READLINK: u64 = 89;
        const SYS_READLINKAT: u64 = 267;
        /// `futex` : l'attente entre fils d'exécution. Il n'y en a qu'un,
        /// donc il n'y a jamais personne à attendre.
        const SYS_FUTEX: u64 = 202;
        const SYS_GETTID: u64 = 186;
        const SYS_GETPPID: u64 = 110;
        const SYS_GETGID: u64 = 104;
        const SYS_GETEGID: u64 = 108;
        /// L'horloge. Elle avance au rythme des instructions exécutées :
        /// c'est le seul temps qui existe ici, et il est reproductible.
        const SYS_CLOCK_GETTIME: u64 = 228;
        const SYS_GETTIMEOFDAY: u64 = 96;
        /// `poll` : une bibliothèque C demande au démarrage si l'entrée
        /// et la sortie sont bien ouvertes.
        const SYS_POLL: u64 = 7;
        /// Les signaux. namsdec n'en délivre aucun — il n'y a ni
        /// terminal, ni horloge, ni voisin pour en envoyer.
        const SYS_RT_SIGACTION: u64 = 13;
        const SYS_RT_SIGPROCMASK: u64 = 14;
        const SYS_SIGALTSTACK: u64 = 131;
        const SYS_CLOSE: u64 = 3;
        const SYS_FCNTL: u64 = 72;
        const SYS_GETCWD: u64 = 79;
        /// Les codes d'erreur que namsdec rend, avec leur nom de la libc.
        const ENOSYS: i64 = -38;
        const EINVAL: i64 = -22;
        const ENOTTY: i64 = -25;
        const ENOMEM: i64 = -12;
        /// Le numéro que namsdec rend à `getpid`, toujours le même : un
        /// programme d'apprentissage doit être reproductible.
        const FAKE_PID: u64 = 1000;

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
            // Rien à faire, et c'est la bonne réponse : il n'y a ni
            // autre fil d'exécution à qui céder la main, ni horloge qui
            // avance pendant qu'on regarde.
            SYS_SCHED_YIELD | SYS_NANOSLEEP => {
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            SYS_GETPID => {
                self.cpu.regs.set64(Reg64::Rax, FAKE_PID);
            }
            SYS_GETUID | SYS_GETEUID => {
                self.cpu.regs.set64(Reg64::Rax, FAKE_PID);
            }
            SYS_ARCH_PRCTL => {
                let code = self.cpu.regs.get64(Reg64::Rdi);
                let valeur = self.cpu.regs.get64(Reg64::Rsi);
                let rendu = match code {
                    ARCH_SET_FS => {
                        self.cpu.regs.fs_base = valeur;
                        0
                    }
                    ARCH_SET_GS => {
                        self.cpu.regs.gs_base = valeur;
                        0
                    }
                    ARCH_GET_FS => {
                        let base = self.cpu.regs.fs_base;
                        self.cpu.mem.write_u64(valeur, base)?;
                        0
                    }
                    ARCH_GET_GS => {
                        let base = self.cpu.regs.gs_base;
                        self.cpu.mem.write_u64(valeur, base)?;
                        0
                    }
                    // `-EINVAL`, comme le noyau : le programme sait quoi
                    // en faire, et cela vaut mieux qu'un arrêt.
                    _ => (-22i64) as u64,
                };
                self.cpu.regs.set64(Reg64::Rax, rendu);
            }
            // Rien à annoncer, et c'est la bonne réponse : il n'y a qu'un
            // fil d'exécution, et il est là.
            SYS_SET_TID_ADDRESS => {
                self.cpu.regs.set64(Reg64::Rax, FAKE_PID);
            }
            SYS_SET_ROBUST_LIST => {
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            // Ces deux-là répondent « c'est fait » sans rien faire :
            // ce qu'ils règlent n'existe pas ici.
            SYS_PRCTL | SYS_PTRACE => {
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            // namsdec ne dédouble rien : il n'a qu'un processus, et un
            // seul fil d'exécution à montrer. Il répond donc zéro — « tu
            // es le fils » — et continue dans la branche du fils.
            //
            // Ce n'est pas arbitraire. Dans ce montage, c'est le fils qui
            // fait le travail : il s'attache, et si l'attache réussit —
            // elle réussit, `ptrace` répond zéro — il poursuit jusqu'au
            // code qu'on est venu lire. Le père, lui, ne saurait
            // qu'attendre un enfant qui n'existe pas.
            //
            // Ce que cela coûte, et il faut le savoir : un programme qui
            // se dédouble pour de bon — un serveur qui traite chaque
            // client dans un fils, un shell qui lance une commande — ne
            // s'exécutera pas comme sur la vraie machine. On ne suit
            // qu'une branche, celle du fils, et le père n'est jamais joué.
            SYS_CLONE | SYS_FORK | SYS_VFORK => {
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            // Un `SIGSTOP` que le programme s'envoie à lui-même : il
            // demande à être mis en pause jusqu'à ce qu'on le relance.
            // namsdec le relance aussitôt — c'est exactement ce que ferait
            // le père, un `PTRACE_CONT` plus loin.
            //
            // Tout autre signal s'arrête et se nomme : un `SIGABRT` ou un
            // `SIGKILL` que l'on avalerait ferait courir un programme que
            // la vraie machine aurait tué, et le montrer faux est pire que
            // ne pas le montrer. De même un signal adressé à un autre
            // processus : il n'y en a pas d'autre ici.
            SYS_KILL | SYS_TKILL | SYS_TGKILL => {
                // Le numéro du signal est le dernier argument, et sa place
                // change avec le nombre d'arguments de l'appel.
                let signal = match number {
                    SYS_KILL | SYS_TKILL => self.cpu.regs.get64(Reg64::Rsi),
                    _ => self.cpu.regs.get64(Reg64::Rdx),
                };
                let cible = self.cpu.regs.get64(Reg64::Rdi);
                if (signal == SIGSTOP || signal == SIGCONT) && cible == FAKE_PID {
                    self.cpu.regs.set64(Reg64::Rax, 0);
                } else {
                    return Err(EmuError::UnsupportedSyscall {
                        backend: Self::NAME,
                        number,
                    });
                }
            }
            // `rseq` : une optimisation que le noyau refuse sans drame
            // quand il ne la gère pas. `-ENOSYS` est exactement ce
            // qu'une bibliothèque C sait encaisser.
            SYS_RSEQ => {
                self.cpu.regs.set64(Reg64::Rax, (-38i64) as u64);
            }
            SYS_GETRANDOM => {
                // L'appel prend (buf, taille, options) : le tampon est
                // dans RDI, la taille dans RSI.
                let buf = self.cpu.regs.get64(Reg64::Rdi);
                let count = self.cpu.regs.get64(Reg64::Rsi);
                let written = self.fill_pseudo_random(buf, count)?;
                self.cpu.regs.set64(Reg64::Rax, written as u64);
            }
            SYS_MMAP => {
                let len = self.cpu.regs.get64(Reg64::Rsi);
                let prot = self.cpu.regs.get64(Reg64::Rdx);
                let flags = self.cpu.regs.get64(Reg64::R10);
                let rendu = if flags & MAP_ANONYMOUS == 0 {
                    // Projeter un fichier demanderait un système de
                    // fichiers. Il n'y en a pas, et l'inventer serait pire
                    // que de répondre non.
                    ENOSYS as u64
                } else {
                    let perms = namsdec_core::Perms {
                        read: prot & PROT_READ != 0,
                        write: prot & PROT_WRITE != 0,
                        execute: prot & PROT_EXEC != 0,
                    };
                    match self.mmap_anonymous(len, perms) {
                        Some(adresse) => adresse,
                        None => ENOMEM as u64,
                    }
                };
                self.cpu.regs.set64(Reg64::Rax, rendu);
            }
            // La mémoire rendue n'est pas reprise : l'arène est un
            // curseur qui monte. Répondre « c'est fait » est exact du
            // point de vue du programme, qui n'y touchera plus.
            SYS_MUNMAP => {
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            // `mprotect`, lui, est appliqué pour de bon : c'est par lui
            // qu'un programme rend ses tables de liaison inscriptibles
            // le temps du chargement, puis les referme. Le voir marcher
            // est la moitié de ce qu'il y a à comprendre sur RELRO.
            SYS_MPROTECT => {
                let addr = self.cpu.regs.get64(Reg64::Rdi);
                let len = self.cpu.regs.get64(Reg64::Rsi);
                let prot = self.cpu.regs.get64(Reg64::Rdx);
                if self.cpu.mem.has_regions() && len > 0 {
                    let début = addr & !(PAGE - 1);
                    let fin = page_up(addr + len);
                    self.cpu.mem.map(
                        début..fin,
                        namsdec_core::Perms {
                            read: prot & PROT_READ != 0,
                            write: prot & PROT_WRITE != 0,
                            execute: prot & PROT_EXEC != 0,
                        },
                        "mprotect",
                    );
                }
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            SYS_FSTAT => {
                let fd = self.cpu.regs.get64(Reg64::Rdi);
                let buf = self.cpu.regs.get64(Reg64::Rsi);
                let rendu = self.fstat(fd, buf)?;
                self.cpu.regs.set64(Reg64::Rax, rendu as u64);
            }
            SYS_NEWFSTATAT => {
                let fd = self.cpu.regs.get64(Reg64::Rdi);
                let buf = self.cpu.regs.get64(Reg64::Rdx);
                let rendu = self.fstat(fd, buf)?;
                self.cpu.regs.set64(Reg64::Rax, rendu as u64);
            }
            // « Es-tu un terminal ? » — non. La bibliothèque en tire la
            // façon de tamponner sa sortie, et `-ENOTTY` est la réponse
            // que donne un noyau pour un tube ou un fichier.
            SYS_IOCTL => {
                self.cpu.regs.set64(Reg64::Rax, ENOTTY as u64);
            }
            SYS_WRITEV | SYS_READV => {
                let fd = self.cpu.regs.get64(Reg64::Rdi);
                let iov = self.cpu.regs.get64(Reg64::Rsi);
                let count = self.cpu.regs.get64(Reg64::Rdx);
                let lecture = number == SYS_READV;
                let rendu = self.vector_io(fd, iov, count, lecture)?;
                self.cpu.regs.set64(Reg64::Rax, rendu as u64);
            }
            SYS_PRLIMIT64 | SYS_GETRLIMIT => {
                // `prlimit64(pid, ressource, neuve, ancienne)` ;
                // `getrlimit(ressource, ancienne)`. Seule la limite de la
                // pile intéresse une bibliothèque C au démarrage.
                let (ressource, ancienne) = if number == SYS_PRLIMIT64 {
                    (
                        self.cpu.regs.get64(Reg64::Rsi),
                        self.cpu.regs.get64(Reg64::R10),
                    )
                } else {
                    (
                        self.cpu.regs.get64(Reg64::Rdi),
                        self.cpu.regs.get64(Reg64::Rsi),
                    )
                };
                if ancienne != 0 {
                    let (courante, maximum) = if ressource == RLIMIT_STACK {
                        // Ce que namsdec a réellement donné à la pile.
                        let pile = self
                            .cpu
                            .mem
                            .regions()
                            .into_iter()
                            .find(|zone| zone.name == "pile")
                            .map(|zone| zone.range.end - zone.range.start)
                            .unwrap_or(0x100000);
                        (pile, pile)
                    } else {
                        (u64::MAX, u64::MAX)
                    };
                    self.cpu.mem.write_u64(ancienne, courante)?;
                    self.cpu.mem.write_u64(ancienne + 8, maximum)?;
                }
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            SYS_UNAME => {
                let buf = self.cpu.regs.get64(Reg64::Rdi);
                self.uname(buf)?;
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            SYS_READLINK | SYS_READLINKAT => {
                self.cpu.regs.set64(Reg64::Rax, EINVAL as u64);
            }
            // `poll` sur les descripteurs ouverts : rien à signaler, et
            // c'est la bonne réponse — une bibliothèque s'en sert au
            // démarrage pour vérifier que l'entrée et la sortie existent,
            // et c'est l'absence de `POLLNVAL` qui le lui dit.
            SYS_POLL => {
                let fds = self.cpu.regs.get64(Reg64::Rdi);
                let combien = self.cpu.regs.get64(Reg64::Rsi).min(64);
                // `struct pollfd` fait huit octets. Un tableau qui ne
                // tient pas en mémoire vaut `-EFAULT`, comme au noyau :
                // c'est une réponse, pas un arrêt.
                if self
                    .cpu
                    .mem
                    .check(fds, (combien * 8) as usize, Access::Write)
                    .is_err()
                {
                    self.cpu.regs.set64(Reg64::Rax, (-14i64) as u64);
                } else {
                    for index in 0..combien {
                        // Seul le dernier champ est à nous : ce qui
                        // arrive, et il n'arrive rien.
                        self.cpu.mem.write_u16(fds + index * 8 + 6, 0)?;
                    }
                    self.cpu.regs.set64(Reg64::Rax, 0);
                }
            }
            // Les signaux : namsdec n'en délivre aucun, il n'y a donc
            // rien à installer — mais refuser l'appel arrêterait le
            // démarrage de toute bibliothèque moderne.
            SYS_RT_SIGACTION | SYS_RT_SIGPROCMASK | SYS_SIGALTSTACK => {
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            // Fermer un descripteur qu'on ne tient pas vraiment : dire
            // que c'est fait est exact du point de vue du programme.
            SYS_CLOSE | SYS_FCNTL => {
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            SYS_GETCWD => {
                // La racine : il n'y a pas de système de fichiers, et
                // prétendre être ailleurs ne servirait personne.
                let buf = self.cpu.regs.get64(Reg64::Rdi);
                let taille = self.cpu.regs.get64(Reg64::Rsi);
                if taille < 2 {
                    self.cpu.regs.set64(Reg64::Rax, (-34i64) as u64);
                } else {
                    self.cpu.mem.poke(buf, b"/\0")?;
                    self.cpu.regs.set64(Reg64::Rax, 2);
                }
            }
            // Personne à attendre : il n'y a qu'un fil d'exécution.
            SYS_FUTEX => {
                self.cpu.regs.set64(Reg64::Rax, 0);
            }
            SYS_GETTID => {
                self.cpu.regs.set64(Reg64::Rax, FAKE_PID);
            }
            SYS_GETPPID => {
                self.cpu.regs.set64(Reg64::Rax, FAKE_PID - 1);
            }
            SYS_GETGID | SYS_GETEGID => {
                self.cpu.regs.set64(Reg64::Rax, FAKE_PID);
            }
            SYS_CLOCK_GETTIME | SYS_GETTIMEOFDAY => {
                // Le temps qui passe ici, c'est le nombre d'instructions
                // exécutées : une horloge reproductible, la seule qui
                // permette de rejouer deux fois la même exécution.
                let buf = if number == SYS_CLOCK_GETTIME {
                    self.cpu.regs.get64(Reg64::Rsi)
                } else {
                    self.cpu.regs.get64(Reg64::Rdi)
                };
                if buf != 0 {
                    let ticks = self.ticks;
                    self.cpu.mem.write_u64(buf, ticks / 1_000_000_000)?;
                    self.cpu.mem.write_u64(buf + 8, ticks % 1_000_000_000)?;
                }
                self.cpu.regs.set64(Reg64::Rax, 0);
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

    /// Remplit un tampon d'octets **reproductibles**.
    ///
    /// Ce n'est pas du hasard, et namsdec ne prétend pas le contraire :
    /// un générateur semé sur une constante, avancé par le nombre
    /// d'instructions déjà exécutées. Deux exécutions du même programme
    /// donnent donc les mêmes octets — ce qui est exactement ce qu'on
    /// veut d'un atelier où l'on revient en arrière, et exactement ce
    /// qu'on ne veut pas d'une clé.
    fn fill_pseudo_random(&mut self, buf: u64, count: u64) -> Result<i64, EmuError> {
        if count == 0 {
            return Ok(0);
        }
        // Xorshift64 : quatre lignes, aucune dépendance, et une suite
        // qui ne se répète pas avant longtemps.
        let mut state = 0x9e37_79b9_7f4a_7c15u64 ^ self.ticks.wrapping_mul(0x2545_f491_4f6c_dd1d);
        let mut bytes = Vec::with_capacity(count as usize);
        for _ in 0..count {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            bytes.push(state as u8);
        }
        self.record_memory(buf, bytes.len());
        self.cpu.mem.write_bytes(buf, &bytes)?;
        Ok(count as i64)
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

    /// RIP est-il posé sur une borne de bibliothèque ?
    fn at_library_boundary(&self) -> bool {
        let rip = self.cpu.regs.rip;
        rip == self.main_return && rip != 0 || self.externals.contains_key(&rip)
    }

    /// Exécute une fonction de bibliothèque fournie par namsdec.
    ///
    /// Elle compte pour **un pas** : c'est ce qu'on veut en avançant à la
    /// main, et c'est ce qui permet de revenir en arrière dessus comme
    /// sur une instruction. Ce qu'elle écrit est journalisé, ce qu'elle
    /// rend passe par RAX, et le retour dépile l'adresse comme le ferait
    /// un `ret`.
    fn call_library(&mut self) -> Result<StepResult, EmuError> {
        let rip = self.cpu.regs.rip;
        let nom = if rip == self.main_return {
            String::new()
        } else {
            self.externals.get(&rip).cloned().unwrap_or_default()
        };

        // Lire l'entrée bloque, comme pour `read` : le pas rend la main,
        // et reprendra quand la ligne sera là.
        if libc::reads_input(&nom) && self.input.is_empty() && !self.input_closed {
            return Ok(StepResult::NeedsInput);
        }

        self.begin_undo();
        self.ticks = self.ticks.wrapping_add(1);
        *self.visits.entry(rip).or_insert(0) += 1;

        let flow = if rip == self.main_return {
            Ok(libc::main_returned(self))
        } else {
            libc::call(self, &nom)
        };
        let flow = match flow {
            Ok(flow) => flow,
            Err(e) => {
                // L'appel n'a pas eu lieu : tout ce qu'il a pu écrire est
                // défait, et RIP reste sur la borne.
                self.step_back();
                self.cpu.regs.rip = rip;
                return Err(e);
            }
        };

        match flow {
            libc::Flow::Returned => self.return_from_library()?,
            libc::Flow::Jumped | libc::Flow::Halted => {}
        }

        Ok(if self.cpu.halted {
            StepResult::Halted
        } else {
            StepResult::Continued
        })
    }

    /// Le retour d'une fonction fournie : exactement ce que fait `ret`.
    fn return_from_library(&mut self) -> Result<(), EmuError> {
        let rsp = self.cpu.regs.get64(Reg64::Rsp);
        let retour = self.cpu.mem.read_u64(rsp)?;
        self.cpu.regs.set64(Reg64::Rsp, rsp.wrapping_add(8));
        self.cpu.regs.rip = retour;
        // La chaîne d'appels se dépile comme pour une vraie fonction —
        // sans vérifier la convention : une fonction écrite en Rust ne
        // peut pas abîmer les registres qu'elle doit rendre intacts.
        if let Some(frame) = self.frames.pop()
            && let Some(undo) = self.history.back_mut()
        {
            undo.frames = FrameChange::Popped(frame);
        }
        Ok(())
    }

    /// Décode puis exécute l'instruction à RIP.
    pub fn step(&mut self) -> Result<StepResult, EmuError> {
        if self.cpu.halted {
            return Ok(StepResult::Halted);
        }

        // Une borne de bibliothèque ? Il n'y a pas d'instruction à
        // décoder là : le chargeur a posé une adresse à la place d'une
        // fonction qui vit ailleurs, et c'est namsdec qui répond. Un
        // appel, un pas — comme un `call` suivi de son `ret`.
        if self.at_library_boundary() {
            return self.call_library();
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

        // Ce que ce pas aura été : su seulement maintenant, RIP ayant
        // bougé ou non, et le journal disant s'il a écrit.
        let wrote = self
            .history
            .back()
            .is_some_and(|undo| !undo.memory.is_empty());
        let kind = classify_step(&instr, rip_before, self.cpu.regs.rip, wrote);
        if let Some(step) = self.trace.back_mut() {
            step.kind = kind;
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
                self.software_breakpoint = Some(instr.ip());
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

            // La pile fantôme (CET) : un second exemplaire des adresses
            // de retour, que le processeur garde hors d'atteinte du
            // programme. namsdec n'en tient pas, et c'est aussi le cas de
            // la plupart des machines sur lesquelles tourne ce code.
            //
            // Sur un processeur sans pile fantôme, ces instructions ne
            // font rien — `rdssp` laisse son registre **inchangé**, et
            // c'est précisément ainsi qu'une bibliothèque C teste la
            // présence du dispositif : elle met zéro, lit, et regarde si
            // c'est toujours zéro. Les traiter comme inconnues arrêtait
            // net tout binaire récent lié statiquement.
            M::Rdsspd | M::Rdsspq | M::Incsspd | M::Incsspq | M::Saveprevssp | M::Clrssbsy => {}

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
            self.trace.pop_front();
        }
        // La trace suit l'historique page pour page : c'est ce qui
        // permet de cliquer sur un pas et d'y revenir vraiment. Sa
        // nature, elle, ne se connaît qu'une fois le pas fait.
        self.trace.push_back(Step {
            addr: self.cpu.regs.rip,
            kind: StepKind::Plain,
        });
        self.history.push_back(Undo {
            regs: self.cpu.regs.clone(),
            flags: self.cpu.flags,
            halted: self.cpu.halted,
            exit_code: self.exit_code,
            software_breakpoint: self.software_breakpoint,
            output_len: self.output.len(),
            consumed_input: Vec::new(),
            input_closed: self.input_closed,
            memory: Vec::new(),
            frames: FrameChange::None,
            heap: self.heap,
            mmap: self.mmap,
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
        // La trace se défait avec l'historique : ce qui n'est plus
        // défaisable n'a plus à figurer dans le chemin parcouru.
        self.trace.pop_back();
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
        self.software_breakpoint = undo.software_breakpoint;
        self.output.truncate(undo.output_len);
        for byte in undo.consumed_input.iter().rev() {
            self.input.push_front(*byte);
        }
        self.input_closed = undo.input_closed;
        self.heap = undo.heap;
        self.mmap = undo.mmap;
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

/// Ce qu'un pas aura été, une fois fait.
///
/// La distinction qui compte est celle du saut : un `jcc` qui ne saute
/// pas est un pas ordinaire, et le montrer comme un saut ferait lire un
/// chemin qui n'a pas été pris.
fn classify_step(instr: &Instruction, before: u64, after: u64, wrote: bool) -> StepKind {
    use iced_x86::FlowControl;
    match instr.flow_control() {
        FlowControl::Call | FlowControl::IndirectCall => return StepKind::Call,
        FlowControl::Return => return StepKind::Return,
        // Pris ou pas : c'est RIP qui le dit, et lui seul.
        FlowControl::UnconditionalBranch
        | FlowControl::IndirectBranch
        | FlowControl::ConditionalBranch
            if after != before.wrapping_add(instr.len() as u64) =>
        {
            return StepKind::Jump;
        }
        _ => {}
    }
    if instr.mnemonic() == Mnemonic::Syscall {
        return StepKind::Syscall;
    }
    if wrote {
        return StepKind::Write;
    }
    StepKind::Plain
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
        // FS et GS, eux, ont une base que le noyau installe. namsdec la
        // pose en chargeant un binaire — c'est là que vivent la mémoire
        // locale au fil et le canari de pile — et un programme peut la
        // déplacer lui-même par `arch_prctl`. Tant que personne ne l'a
        // installée, elle vaut zéro : rendre `None` dit alors « je ne
        // sais pas où pointe FS », ce qui vaut mieux qu'une adresse
        // inventée.
        Register::FS => (regs.fs_base != 0).then_some(regs.fs_base),
        Register::GS => (regs.gs_base != 0).then_some(regs.gs_base),
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
