//! Les petites énumérations d'état de l'interface : onglets, modes,
//! formats, et le statut courant. Chacune porte son identifiant stable
//! (distinct du libellé, qui change avec la langue) et son libellé.
//!
//! Sorties de `main.rs` pour l'alléger — ce sont des types de valeur,
//! sans lien avec le reste.

use super::*;

/// Les vues du panneau latéral qui s'échangent.
///
/// Registres et flags n'en font plus partie : ce sont eux qu'on surveille
/// en avançant instruction par instruction, ils ont donc un bandeau fixe
/// en haut du panneau plutôt qu'un onglet où il faut revenir. Et la
/// console a quitté le panneau pour le bas de la fenêtre, où elle a la
/// largeur d'un terminal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SidebarTab {
    Stack,
    Memory,
    Heap,
    /// Ce que chaque ligne a produit comme octets.
    Bytes,
    Re,
}

impl SidebarTab {
    pub(crate) const ALL: [SidebarTab; 5] = [
        SidebarTab::Stack,
        SidebarTab::Memory,
        SidebarTab::Heap,
        SidebarTab::Bytes,
        SidebarTab::Re,
    ];

    /// L'identifiant qui part dans les réglages.
    ///
    /// Stable, et distinct du libellé : celui-ci change avec la langue,
    /// et un fichier écrit en français ne doit pas devenir illisible
    /// parce qu'on a basculé en anglais.
    pub(crate) fn id(self) -> &'static str {
        match self {
            SidebarTab::Stack => "stack",
            SidebarTab::Memory => "memory",
            SidebarTab::Heap => "heap",
            SidebarTab::Bytes => "bytes",
            SidebarTab::Re => "re",
        }
    }

    /// L'onglet que cet identifiant désigne, s'il en désigne un.
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        SidebarTab::ALL.into_iter().find(|tab| tab.id() == id)
    }

    pub(crate) fn label(self, s: &Strings) -> &'static str {
        match self {
            SidebarTab::Stack => s.tab_stack,
            SidebarTab::Memory => s.tab_memory,
            SidebarTab::Heap => s.tab_heap,
            SidebarTab::Bytes => s.tab_bytes,
            SidebarTab::Re => s.tab_re,
        }
    }
}

/// Jusqu'où court « aller au prochain… ».
///
/// Trois questions, celles qu'on se pose en explorant un programme
/// inconnu : qu'appelle-t-il, quand sort-il d'ici, que demande-t-il au
/// noyau ? `nextcall`, `nextret` et `nextsyscall` chez pwndbg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UntilKind {
    Call,
    Return,
    Syscall,
}

impl UntilKind {
    /// Cette instruction est-elle de celles qu'on cherche ?
    pub(crate) fn matches(self, instr: &namsdec_disasm::Decoded) -> bool {
        use namsdec_disasm::FlowControl;
        match self {
            // L'appel indirect compte autant que l'autre : `call rax`
            // est précisément celui qu'on veut voir arriver.
            UntilKind::Call => matches!(
                instr.flow_control(),
                FlowControl::Call | FlowControl::IndirectCall
            ),
            UntilKind::Return => instr.flow_control() == FlowControl::Return,
            UntilKind::Syscall => instr.mnemonic() == namsdec_disasm::Mnemonic::Syscall,
        }
    }
}

/// Ce qu'une pastille de fenêtre demande au système.
///
/// La fermeture n'y est pas : elle passe par la garde du travail non
/// enregistré, comme « Quitter ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowCommand {
    Minimize,
    Maximize,
}

/// Sous quelle forme on écrit ce qu'on cherche en mémoire.
///
/// Trois, parce que ce sont les trois choses qu'on a sous les yeux : un
/// texte qu'on vient d'écrire, des octets relevés dans un vidage, une
/// valeur lue dans un registre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchKind {
    Text,
    Bytes,
    Value,
}

impl SearchKind {
    pub(crate) const ALL: [SearchKind; 3] =
        [SearchKind::Text, SearchKind::Bytes, SearchKind::Value];

    pub(crate) fn label(self, s: &Strings) -> &'static str {
        match self {
            SearchKind::Text => s.search_text,
            SearchKind::Bytes => s.search_bytes,
            SearchKind::Value => s.search_value,
        }
    }

    /// Les octets à chercher, tels que la mémoire les porte.
    ///
    /// Une valeur est cherchée en petit-boutiste — l'ordre dans lequel
    /// le processeur l'a écrite — et sur ses octets utiles seulement :
    /// chercher `1` sur huit octets ne trouverait qu'un mot aligné, là
    /// où l'on veut l'octet.
    pub(crate) fn needle(self, saisie: &str) -> Option<Vec<u8>> {
        let saisie = saisie.trim();
        if saisie.is_empty() {
            return None;
        }
        match self {
            SearchKind::Text => Some(saisie.as_bytes().to_vec()),
            SearchKind::Bytes => {
                let propre: String = saisie
                    .chars()
                    .filter(|c| !c.is_whitespace() && *c != ',' && *c != '-')
                    .collect();
                let propre = propre.strip_prefix("0x").unwrap_or(&propre).to_owned();
                if propre.is_empty() || !propre.len().is_multiple_of(2) {
                    return None;
                }
                (0..propre.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&propre[i..i + 2], 16).ok())
                    .collect()
            }
            SearchKind::Value => {
                let value = match saisie
                    .strip_prefix("0x")
                    .or_else(|| saisie.strip_prefix("0X"))
                {
                    Some(hex) => u64::from_str_radix(hex, 16).ok()?,
                    None => saisie.parse::<u64>().ok()?,
                };
                let bytes = value.to_le_bytes();
                let utiles = bytes.iter().rposition(|&b| b != 0).map_or(1, |i| i + 1);
                Some(bytes[..utiles].to_vec())
            }
        }
    }
}

/// Ce que l'onglet Mémoire suit, pas à pas.
///
/// Sans cela, regarder la pile pendant qu'on avance demande de retaper
/// l'adresse à chaque instruction. pwndbg résout la même chose en
/// réaffichant son contexte à chaque arrêt ; ici, on choisit ce qu'on
/// veut voir bouger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemFollow {
    /// L'adresse reste où on l'a mise.
    Fixed,
    Rsp,
    Rip,
}

impl MemFollow {
    pub(crate) const ALL: [MemFollow; 3] = [MemFollow::Fixed, MemFollow::Rsp, MemFollow::Rip];

    pub(crate) fn label(self, s: &Strings) -> &'static str {
        match self {
            MemFollow::Fixed => s.memory_follow_none,
            MemFollow::Rsp => "RSP",
            MemFollow::Rip => "RIP",
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            MemFollow::Fixed => "fixed",
            MemFollow::Rsp => "rsp",
            MemFollow::Rip => "rip",
        }
    }

    pub(crate) fn by_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|kind| kind.id() == id)
            .unwrap_or(MemFollow::Fixed)
    }

    /// L'adresse à regarder maintenant, alignée sur seize octets pour
    /// que le vidage ne danse pas d'une colonne à l'autre.
    pub(crate) fn address(self, regs: &namsdec_core::Registers) -> Option<u64> {
        let value = match self {
            MemFollow::Fixed => return None,
            MemFollow::Rsp => regs.get64(Reg64::Rsp),
            MemFollow::Rip => regs.rip,
        };
        Some(value & !0xf)
    }
}

/// La base dans laquelle la calculatrice lit et écrit.
///
/// Elle gouverne les deux : le grand résultat est écrit dedans, et un
/// chiffre tapé au clavier de l'écran y entre avec le préfixe qui va
/// avec. C'est ce que fait la calculatrice de GNOME, et c'est ce qu'on
/// attend — choisir « hexadécimal » puis taper `1f` doit donner
/// trente et un, non mille neuf cent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CalcBase {
    Hex,
    Dec,
    Oct,
    Bin,
}

impl CalcBase {
    pub(crate) const ALL: [CalcBase; 4] =
        [CalcBase::Hex, CalcBase::Dec, CalcBase::Oct, CalcBase::Bin];

    pub(crate) fn id(self) -> &'static str {
        match self {
            CalcBase::Hex => "hex",
            CalcBase::Dec => "dec",
            CalcBase::Oct => "oct",
            CalcBase::Bin => "bin",
        }
    }

    pub(crate) fn by_id(id: &str) -> Self {
        CalcBase::ALL
            .into_iter()
            .find(|b| b.id() == id)
            .unwrap_or(CalcBase::Hex)
    }

    pub(crate) fn label(self, s: &Strings) -> &'static str {
        match self {
            CalcBase::Hex => s.reading_hex,
            CalcBase::Dec => s.reading_unsigned,
            CalcBase::Oct => s.reading_octal,
            CalcBase::Bin => s.calculator_base_binary,
        }
    }

    /// Ce qu'on écrit devant un nombre pour que l'évaluateur le lise
    /// dans cette base. Rien pour le décimal, qui est le défaut de tout
    /// le monde.
    pub(crate) fn prefix(self) -> &'static str {
        match self {
            CalcBase::Hex => "0x",
            CalcBase::Dec => "",
            CalcBase::Oct => "0o",
            CalcBase::Bin => "0b",
        }
    }

    /// Ce chiffre a-t-il un sens dans cette base ?
    ///
    /// Sert à éteindre les touches plutôt qu'à les laisser produire une
    /// expression que l'évaluateur refusera : un clavier qui propose `9`
    /// en binaire ment sur ce qu'on peut taper.
    pub(crate) fn accepte(self, c: char) -> bool {
        let valeur = match c.to_ascii_lowercase() {
            c if c.is_ascii_digit() => c as u32 - '0' as u32,
            c if ('a'..='f').contains(&c) => c as u32 - 'a' as u32 + 10,
            _ => return false,
        };
        valeur
            < match self {
                CalcBase::Hex => 16,
                CalcBase::Dec => 10,
                CalcBase::Oct => 8,
                CalcBase::Bin => 2,
            }
    }

    /// Une valeur écrite dans cette base, préfixe compris.
    pub(crate) fn écrire(self, valeur: u64) -> String {
        match self {
            CalcBase::Hex => format!("{valeur:#x}"),
            CalcBase::Dec => valeur.to_string(),
            CalcBase::Oct => format!("{valeur:#o}"),
            CalcBase::Bin => format!("{valeur:#b}"),
        }
    }
}

/// Sur combien de bits la calculatrice montre son résultat.
///
/// Un `and eax, 0x1f` travaille sur trente-deux bits ; lire le résultat
/// sur soixante-quatre fait compter des zéros qui ne veulent rien dire.
/// La largeur masque l'affichage — elle ne change pas le calcul, qui se
/// fait toujours sur soixante-quatre bits, comme le processeur.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CalcWidth {
    W8,
    W16,
    W32,
    W64,
}

impl CalcWidth {
    pub(crate) const ALL: [CalcWidth; 4] = [
        CalcWidth::W8,
        CalcWidth::W16,
        CalcWidth::W32,
        CalcWidth::W64,
    ];

    pub(crate) fn bits(self) -> u32 {
        match self {
            CalcWidth::W8 => 8,
            CalcWidth::W16 => 16,
            CalcWidth::W32 => 32,
            CalcWidth::W64 => 64,
        }
    }

    pub(crate) fn id(self) -> String {
        self.bits().to_string()
    }

    pub(crate) fn by_id(id: &str) -> Self {
        CalcWidth::ALL
            .into_iter()
            .find(|w| w.id() == id)
            .unwrap_or(CalcWidth::W64)
    }

    /// Le masque de cette largeur. `u64::MAX` pour soixante-quatre :
    /// `1 << 64` déborderait.
    pub(crate) fn mask(self) -> u64 {
        match self {
            CalcWidth::W64 => u64::MAX,
            autre => (1u64 << autre.bits()) - 1,
        }
    }
}

/// Comment lire les octets de l'onglet Mémoire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MemoryFormat {
    /// Le vidage hexadécimal classique, seize octets par ligne.
    Bytes,
    /// Des mots de huit octets, en décimal à côté : la vue d'une pile.
    Words,
    /// Le texte jusqu'au premier octet nul.
    Text,
    /// Les pointeurs suivis jusqu'au bout : à cette adresse il y a cette
    /// adresse, où il y a cette chaîne. Le geste que pwndbg appelle
    /// *telescope*, et qu'on fait sinon à la main, une lecture après
    /// l'autre.
    Chain,
}

impl MemoryFormat {
    pub(crate) const ALL: [MemoryFormat; 4] = [
        MemoryFormat::Bytes,
        MemoryFormat::Words,
        MemoryFormat::Text,
        MemoryFormat::Chain,
    ];

    pub(crate) fn label(self, s: &Strings) -> &'static str {
        match self {
            MemoryFormat::Bytes => s.memory_format_bytes,
            MemoryFormat::Words => s.memory_format_words,
            MemoryFormat::Text => s.memory_format_text,
            MemoryFormat::Chain => s.memory_format_chain,
        }
    }

    /// L'identifiant écrit dans `settings.json`.
    pub(crate) fn id(self) -> &'static str {
        match self {
            MemoryFormat::Bytes => "bytes",
            MemoryFormat::Words => "words",
            MemoryFormat::Text => "text",
            MemoryFormat::Chain => "chain",
        }
    }

    pub(crate) fn by_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|format| format.id() == id)
            .unwrap_or(MemoryFormat::Bytes)
    }
}

/// Les deux vues du panneau bas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ConsoleTab {
    /// Ce que le programme a écrit lui-même, via `write` — la seule chose
    /// qu'il affiche au monde, et donc celle qu'on vient vérifier après
    /// l'avoir exécuté.
    Output,
    /// Ce que nasm a eu à dire de la source : erreurs et avertissements,
    /// reliés à leur ligne. Ils vivaient sous la barre de statut, où une
    /// liste un peu longue n'avait pas la place d'être lue.
    Diagnostics,
    /// Où le programme a passé son temps : les lignes classées par
    /// nombre de passages. La bande de chaleur dans la gouttière dit
    /// « c'est chaud par là » ; ce classement dit laquelle est la plus
    /// chaude, de combien, et y emmène d'un clic.
    Profile,
}

impl ConsoleTab {
    pub(crate) const ALL: [ConsoleTab; 3] = [
        ConsoleTab::Output,
        ConsoleTab::Diagnostics,
        ConsoleTab::Profile,
    ];

    pub(crate) fn label(self, s: &Strings) -> &'static str {
        match self {
            ConsoleTab::Output => s.tab_output,
            ConsoleTab::Diagnostics => s.tab_diagnostics,
            ConsoleTab::Profile => s.tab_profile,
        }
    }
}

/// `Debug` sert aux tests, qui doivent pouvoir dire quel statut ils ont
/// obtenu quand ils en attendaient un autre.
#[derive(Debug)]
pub(crate) enum Status {
    Idle,
    AssembleOk,
    AssembleFailed,
    /// Une zone surveillée a changé de valeur.
    Watchpoint {
        addr: u64,
        old: u64,
        new: u64,
    },
    /// Un registre surveillé a changé : la question « qui écrit dans
    /// RAX ? » a sa réponse à l'écran.
    RegisterChanged {
        register: &'static str,
        old: u64,
        new: u64,
    },
    /// Le dernier pas vient d'être défait.
    SteppedBack,
    /// Il n'y avait plus rien à défaire.
    NothingToUndo,
    Running,
    Halted,
    /// Le programme s'est terminé lui-même par `exit`, avec ce code — ce
    /// qui n'est pas la même chose que la machine arrêtée par `hlt`.
    Exited {
        code: i64,
    },
    /// Le programme attend son entrée standard : il ne reprendra que
    /// lorsqu'on lui aura donné de quoi lire.
    WaitingForInput,
    BinaryLoaded {
        bytes: usize,
        truncated: bool,
    },
    Breakpoint(u64),
    /// On vient d'emmener le listing à cette adresse, depuis un panneau
    /// d'analyse.
    ///
    /// Le dire plutôt que de laisser le silence : le clic ferme la
    /// fenêtre d'analyse et déplace le listing derrière elle. Sans
    /// phrase, on ne sait pas si l'on est arrivé quelque part ou si la
    /// fenêtre s'est fermée toute seule.
    WentTo(u64),
    StepLimit,
    NasmMissing,
    /// Une confirmation ordinaire : quelque chose a été fait, et il n'y
    /// a rien de plus à en dire qu'une phrase.
    ///
    /// De quoi retrouver la phrase, et non la phrase : celle-ci était
    /// figée dans la langue du moment où la note avait été posée, si
    /// bien qu'après un changement de langue la barre de statut gardait
    /// l'ancienne — du français dans une interface anglaise, et
    /// l'inverse. On garde le chemin vers le texte ; il se lit à
    /// l'affichage, dans la langue d'alors.
    Note(fn(&Strings) -> &'static str),
    /// Un nombre et ce qu'il compte, la phrase venant des textes : le
    /// nombre ne se traduit pas, la phrase si.
    Counted(usize, fn(&Strings) -> &'static str),
    Error(String),
}

/// Mode d'édition façon (n)vim : en Normal, l'éditeur ne prend pas la
/// frappe comme du texte (`i`/`I` pour passer en Insertion) ; en
/// Insertion, il se comporte comme un éditeur de texte classique
/// (`Échap` pour revenir en Normal). Volontairement limité à ces deux
/// modes et ces trois touches — pas les mouvements hjkl ni les commandes
/// (dd, yy, ...) de vim, qui n'ont pas été demandés.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(crate) enum EditorMode {
    #[default]
    Normal,
    Insert,
}

impl EditorMode {
    pub(crate) fn label(self) -> &'static str {
        // Comme dans (n)vim lui-même : jamais traduit, y compris en UI
        // anglaise — c'est la convention, pas un texte applicatif.
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
        }
    }

    pub(crate) fn color(self, theme: &Theme) -> egui::Color32 {
        match self {
            EditorMode::Normal => theme.accent,
            EditorMode::Insert => theme.good,
        }
    }
}
