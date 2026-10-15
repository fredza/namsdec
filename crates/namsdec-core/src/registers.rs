/// Les 16 registres généraux x86-64, dans l'ordre d'encodage ModRM/REX
/// (0=RAX .. 7=RDI, 8=R8 .. 15=R15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum Reg64 {
    Rax,
    Rcx,
    Rdx,
    Rbx,
    Rsp,
    Rbp,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

/// Les seize registres vectoriels, larges de 256 bits depuis AVX.
///
/// Le même registre porte deux noms selon ce qu'on en prend : `xmm3`
/// désigne la moitié basse, `ymm3` le tout. Ce sont bien les mêmes bits,
/// et c'est ce qui rend la règle d'AVX si facile à oublier — une
/// instruction VEX remet la moitié haute à zéro, une instruction SSE la
/// laisse intacte.
///
/// C'est là que se fait tout le calcul flottant du x86-64 moderne : la
/// pile du 8087 n'est plus utilisée que par du code ancien, et l'ABI
/// System V passe les flottants dans XMM0 à XMM7.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum RegXmm {
    Xmm0,
    Xmm1,
    Xmm2,
    Xmm3,
    Xmm4,
    Xmm5,
    Xmm6,
    Xmm7,
    Xmm8,
    Xmm9,
    Xmm10,
    Xmm11,
    Xmm12,
    Xmm13,
    Xmm14,
    Xmm15,
    // Les seize suivants n'existent qu'avec AVX-512 : son encodage
    // EVEX a quatre bits pour nommer un registre là où VEX n'en avait
    // que trois. Un binaire qui ne s'en sert pas les laisse à zéro.
    Xmm16,
    Xmm17,
    Xmm18,
    Xmm19,
    Xmm20,
    Xmm21,
    Xmm22,
    Xmm23,
    Xmm24,
    Xmm25,
    Xmm26,
    Xmm27,
    Xmm28,
    Xmm29,
    Xmm30,
    Xmm31,
}

/// Les huit registres de MASQUE d'AVX-512, `k0` à `k7`.
///
/// Un bit par voie du vecteur : c'est ainsi qu'AVX-512 fait ce que les
/// jeux précédents faisaient avec un vecteur de zéros et de uns. Une
/// comparaison n'écrit plus un masque dans un registre vectoriel, elle
/// écrit UN BIT PAR ÉLÉMENT ici — et l'instruction suivante s'en sert
/// pour ne travailler que sur les voies retenues.
///
/// `k0` a une particularité : employé comme masque d'opérande, il veut
/// dire « aucun masque, toutes les voies ». Il reste un registre
/// ordinaire quand on l'écrit ou qu'on le lit explicitement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum RegMask {
    K0,
    K1,
    K2,
    K3,
    K4,
    K5,
    K6,
    K7,
}

impl RegMask {
    pub const ALL: [RegMask; 8] = [
        RegMask::K0,
        RegMask::K1,
        RegMask::K2,
        RegMask::K3,
        RegMask::K4,
        RegMask::K5,
        RegMask::K6,
        RegMask::K7,
    ];

    pub fn name(self) -> &'static str {
        match self {
            RegMask::K0 => "k0",
            RegMask::K1 => "k1",
            RegMask::K2 => "k2",
            RegMask::K3 => "k3",
            RegMask::K4 => "k4",
            RegMask::K5 => "k5",
            RegMask::K6 => "k6",
            RegMask::K7 => "k7",
        }
    }
}

impl RegXmm {
    pub const ALL: [RegXmm; 32] = [
        RegXmm::Xmm0,
        RegXmm::Xmm1,
        RegXmm::Xmm2,
        RegXmm::Xmm3,
        RegXmm::Xmm4,
        RegXmm::Xmm5,
        RegXmm::Xmm6,
        RegXmm::Xmm7,
        RegXmm::Xmm8,
        RegXmm::Xmm9,
        RegXmm::Xmm10,
        RegXmm::Xmm11,
        RegXmm::Xmm12,
        RegXmm::Xmm13,
        RegXmm::Xmm14,
        RegXmm::Xmm15,
        RegXmm::Xmm16,
        RegXmm::Xmm17,
        RegXmm::Xmm18,
        RegXmm::Xmm19,
        RegXmm::Xmm20,
        RegXmm::Xmm21,
        RegXmm::Xmm22,
        RegXmm::Xmm23,
        RegXmm::Xmm24,
        RegXmm::Xmm25,
        RegXmm::Xmm26,
        RegXmm::Xmm27,
        RegXmm::Xmm28,
        RegXmm::Xmm29,
        RegXmm::Xmm30,
        RegXmm::Xmm31,
    ];

    /// Le numéro du registre, tel qu'il s'écrit : `xmm5` pour 5.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// La pile du coprocesseur 8087 : huit registres en anneau, désignés
/// *relativement* au sommet — `st0` est le sommet, `st1` celui d'en
/// dessous, et un `fld` fait tout descendre d'un cran.
///
/// Les valeurs sont gardées en double précision, là où un vrai x87
/// calcule sur 80 bits. C'est une approximation, et elle est assumée :
/// simuler le format étendu demanderait une arithmétique logicielle
/// entière, pour un écart qui n'apparaît qu'au-delà du quinzième chiffre.
/// Ce que le x87 enseigne — une pile au lieu de registres nommés, des
/// comparaisons qui répondent dans un mot d'état — est intact.
#[derive(Clone, Copy, Debug, Default)]
pub struct X87 {
    /// Les huit cases, dans l'ordre physique.
    slots: [f64; 8],
    /// Laquelle est le sommet.
    top: usize,
    /// Une case vide n'est pas une case à zéro : `fld` sur une pile
    /// pleine est une erreur, et un débogueur doit pouvoir le montrer.
    occupied: [bool; 8],
    /// Les quatre bits de condition du mot d'état (C0, C1, C2, C3), là
    /// où `fcom` range le résultat d'une comparaison et d'où `fstsw ax`
    /// le tire.
    pub status: u16,
}

impl X87 {
    /// Le masque des quatre bits de condition dans le mot d'état.
    pub const C0: u16 = 1 << 8;
    pub const C1: u16 = 1 << 9;
    pub const C2: u16 = 1 << 10;
    pub const C3: u16 = 1 << 14;

    /// La case physique derrière `st(i)`.
    fn slot(&self, i: usize) -> usize {
        (self.top + i) % 8
    }

    /// Le contenu de `st(i)`. Une case vide rend zéro plutôt que de
    /// paniquer : l'émulateur signale la pile vide autrement.
    pub fn get(&self, i: usize) -> f64 {
        self.slots[self.slot(i)]
    }

    pub fn set(&mut self, i: usize, value: f64) {
        let slot = self.slot(i);
        self.slots[slot] = value;
        self.occupied[slot] = true;
    }

    pub fn is_occupied(&self, i: usize) -> bool {
        self.occupied[self.slot(i)]
    }

    /// Combien de cases sont prises.
    pub fn depth(&self) -> usize {
        self.occupied.iter().filter(|o| **o).count()
    }

    /// Empile : le sommet descend d'un cran, et la valeur s'y écrit.
    pub fn push(&mut self, value: f64) {
        self.top = (self.top + 7) % 8;
        self.slots[self.top] = value;
        self.occupied[self.top] = true;
    }

    /// Dépile le sommet et le rend.
    pub fn pop(&mut self) -> f64 {
        let value = self.slots[self.top];
        self.occupied[self.top] = false;
        self.top = (self.top + 1) % 8;
        value
    }

    /// Fait tourner l'anneau sans rien écrire (`fincstp`/`fdecstp`).
    pub fn rotate(&mut self, forward: bool) {
        self.top = if forward {
            (self.top + 1) % 8
        } else {
            (self.top + 7) % 8
        };
    }

    /// Marque une case comme libre, sans déplacer le sommet (`ffree`).
    pub fn free(&mut self, i: usize) {
        let slot = self.slot(i);
        self.occupied[slot] = false;
    }

    /// Le mot d'état tel que `fstsw` le rend : les bits de condition,
    /// plus le numéro du sommet sur trois bits.
    pub fn status_word(&self) -> u16 {
        self.status | ((self.top as u16) << 11)
    }

    /// Remet le coprocesseur à neuf : pile vide, conditions effacées.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Debug, Default)]
pub struct Registers {
    gpr: [u64; 16],
    /// Les registres vectoriels, en quatre tranches de 128 bits.
    ///
    /// La première tranche est `xmm`, les deux premières `ymm`, les
    /// quatre `zmm` : trois fenêtres sur les mêmes bits, et c'est
    /// l'instruction qui dit combien elle en lit. Ils ne se mélangent
    /// pas aux généraux — ni les mêmes instructions, ni les mêmes
    /// tailles, ni le même usage. Trente-deux et non seize, parce que
    /// l'encodage EVEX d'AVX-512 a quatre bits pour nommer un registre
    /// là où VEX n'en avait que trois ; un binaire qui n'en emploie
    /// pas les laisse à zéro.
    xmm: [[u128; 4]; 32],
    /// Les huit registres de masque d'AVX-512. Soixante-quatre bits
    /// chacun — assez pour une voie par octet d'un registre de 512.
    masques: [u64; 8],
    /// La pile du 8087, que les programmes d'apprentissage utilisent
    /// encore — c'est par elle que la plupart des cours font leur premier
    /// calcul à virgule.
    pub x87: X87,
    pub rip: u64,
    /// La base du segment FS : ce que le noyau installe pour désigner la
    /// mémoire locale au fil d'exécution.
    ///
    /// En 64 bits, CS, DS, ES et SS ont tous une base nulle — c'est le
    /// modèle plat. FS et GS font exception, et c'est *tout* leur intérêt :
    /// `mov rax, fs:[0x28]` ne lit pas l'adresse 0x28, mais l'adresse
    /// 0x28 **au-dessus de la base du fil**, là où vit le canari de pile.
    ///
    /// Un programme qui vient d'un compilateur moderne le fait dès sa
    /// première fonction protégée.
    ///
    /// Nulle tant que personne ne l'a installée — un programme écrit à la
    /// main n'en a pas besoin, et une base nulle fait dire à namsdec
    /// qu'il ne sait pas où pointe FS plutôt que de lire n'importe où.
    pub fs_base: u64,
    /// La base du segment GS. Linux la réserve au noyau ; un programme en
    /// espace utilisateur ne s'en sert presque jamais.
    pub gs_base: u64,
}

impl Registers {
    pub fn new() -> Self {
        Self::default()
    }

    /// La moitié basse — celle que nomme `xmm` : 128 bits, deux flottants
    /// doubles ou quatre simples selon ce qu'on y a mis.
    pub fn get_xmm(&self, r: RegXmm) -> u128 {
        self.xmm[r as usize][0]
    }

    /// Écrit la moitié basse **sans toucher à la haute** : c'est ce que
    /// fait une instruction SSE, et c'est de là que vient le coût des
    /// transitions entre code SSE et code AVX sur un vrai processeur.
    pub fn set_xmm(&mut self, r: RegXmm, v: u128) {
        self.xmm[r as usize][0] = v;
    }

    /// Un registre de masque.
    pub fn get_mask(&self, r: RegMask) -> u64 {
        self.masques[r as usize]
    }

    /// Écrit un registre de masque.
    ///
    /// Les soixante-quatre bits, sans égard à la largeur de
    /// l'instruction qui écrit : `kmovb` n'en pose que huit, et c'est à
    /// elle de masquer avant d'appeler. Un registre tronqué ici ferait
    /// perdre silencieusement ce qu'une instruction plus large y avait
    /// mis.
    pub fn set_mask(&mut self, r: RegMask, v: u64) {
        self.masques[r as usize] = v;
    }

    /// Les 256 bits de bas, moitié basse d'abord.
    pub fn get_ymm(&self, r: RegXmm) -> [u128; 2] {
        let z = self.xmm[r as usize];
        [z[0], z[1]]
    }

    /// Écrit les 256 bits de bas, et REMET LE HAUT À ZÉRO.
    ///
    /// C'est la règle de VEX, et elle vaut ici aussi : une instruction
    /// AVX qui écrit `ymm0` efface les bits 256 à 511 de `zmm0`. Les
    /// garder ferait réapparaître, dans un code mêlé, des octets qu'un
    /// vrai processeur a effacés.
    pub fn set_ymm(&mut self, r: RegXmm, v: [u128; 2]) {
        self.xmm[r as usize] = [v[0], v[1], 0, 0];
    }

    /// Le registre entier — les 512 bits d'AVX-512, tranche basse
    /// d'abord.
    pub fn get_zmm(&self, r: RegXmm) -> [u128; 4] {
        self.xmm[r as usize]
    }

    pub fn set_zmm(&mut self, r: RegXmm, v: [u128; 4]) {
        self.xmm[r as usize] = v;
    }

    pub fn get64(&self, r: Reg64) -> u64 {
        self.gpr[r as usize]
    }

    pub fn get32(&self, r: Reg64) -> u32 {
        self.gpr[r as usize] as u32
    }

    pub fn get16(&self, r: Reg64) -> u16 {
        self.gpr[r as usize] as u16
    }

    pub fn get8l(&self, r: Reg64) -> u8 {
        self.gpr[r as usize] as u8
    }

    pub fn set64(&mut self, r: Reg64, v: u64) {
        self.gpr[r as usize] = v;
    }

    /// Écrire un registre 32 bits met à zéro les 32 bits hauts du registre
    /// 64 bits qui le contient — c'est le comportement réel de x86-64,
    /// source classique de confusion pour qui apprend l'assembleur.
    pub fn set32(&mut self, r: Reg64, v: u32) {
        self.gpr[r as usize] = v as u64;
    }

    /// Écrire un registre 16 bits préserve le reste du registre 64 bits
    /// (comportement historique hérité du x86 16 bits).
    pub fn set16(&mut self, r: Reg64, v: u16) {
        let slot = &mut self.gpr[r as usize];
        *slot = (*slot & !0xFFFF) | v as u64;
    }

    /// L'octet *haut* du registre bas — AH, BH, CH, DH : les bits 8 à 15.
    ///
    /// Un vestige du 8086, qu'une instruction ne peut atteindre que sans
    /// préfixe REX : `mov ah, 1` s'assemble, `mov ah, r8b` non.
    pub fn get8h(&self, r: Reg64) -> u8 {
        (self.gpr[r as usize] >> 8) as u8
    }

    /// Écrire l'octet haut préserve le reste du registre 64 bits.
    pub fn set8h(&mut self, r: Reg64, v: u8) {
        let slot = &mut self.gpr[r as usize];
        *slot = (*slot & !0xFF00) | ((v as u64) << 8);
    }

    /// Écrire l'octet bas préserve le reste du registre 64 bits.
    pub fn set8l(&mut self, r: Reg64, v: u8) {
        let slot = &mut self.gpr[r as usize];
        *slot = (*slot & !0xFF) | v as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set32_zero_extends_upper_bits() {
        let mut regs = Registers::new();
        regs.set64(Reg64::Rax, 0xDEAD_BEEF_0000_0001);
        regs.set32(Reg64::Rax, 0x1234_5678);
        assert_eq!(regs.get64(Reg64::Rax), 0x0000_0000_1234_5678);
    }

    #[test]
    fn set16_preserves_upper_bits() {
        let mut regs = Registers::new();
        regs.set64(Reg64::Rax, 0x1122_3344_5566_7788);
        regs.set16(Reg64::Rax, 0xBEEF);
        assert_eq!(regs.get64(Reg64::Rax), 0x1122_3344_5566_BEEF);
    }

    #[test]
    fn set8l_preserves_upper_bits() {
        let mut regs = Registers::new();
        regs.set64(Reg64::Rax, 0x1122_3344_5566_7788);
        regs.set8l(Reg64::Rax, 0xAB);
        assert_eq!(regs.get64(Reg64::Rax), 0x1122_3344_5566_77AB);
    }

    #[test]
    fn rax_and_r8_are_independent_slots() {
        let mut regs = Registers::new();
        regs.set64(Reg64::Rax, 1);
        regs.set64(Reg64::R8, 2);
        assert_eq!(regs.get64(Reg64::Rax), 1);
        assert_eq!(regs.get64(Reg64::R8), 2);
    }
}
