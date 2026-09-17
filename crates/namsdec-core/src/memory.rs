use std::ops::Range;
use thiserror::Error;

/// Les erreurs des crates métier portent un `Display` **anglais** et
/// gardent leurs valeurs (ici l'adresse) dans des variantes structurées.
///
/// L'anglais est la langue des traces et des tests, celle qui se colle
/// dans un rapport de bug ; la langue de l'utilisateur, elle, est une
/// affaire d'interface, et c'est le module `errors` de `namsdec-gui` qui
/// traduit chaque variante. Aucune crate sous l'interface n'a donc à
/// connaître la langue choisie.
#[derive(Debug, Error, PartialEq, Eq, Clone, Copy)]
pub enum MemError {
    #[error("memory access out of bounds at {addr:#x}")]
    OutOfBounds { addr: u64 },
    /// L'adresse est dans la fenêtre simulée, mais aucune zone n'y est
    /// projetée : ni programme, ni pile, ni tas. Sur un vrai système,
    /// c'est le défaut de segmentation.
    #[error("nothing mapped at {addr:#x}")]
    Unmapped { addr: u64 },
    /// La zone existe, mais pas avec ce droit-là : écrire dans le code,
    /// exécuter des données.
    #[error("{needed} not allowed at {addr:#x} (in {region})")]
    Protection {
        addr: u64,
        needed: Access,
        region: &'static str,
    },
}

/// Ce qu'un accès demande à la mémoire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    Execute,
}

impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // En anglais comme le reste des erreurs métier ; l'interface
        // traduit.
        let word = match self {
            Access::Read => "read",
            Access::Write => "write",
            Access::Execute => "execute",
        };
        f.write_str(word)
    }
}

/// Les droits d'une zone, comme ceux d'un segment de programme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Perms {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Perms {
    /// Tout permis — l'état d'un bloc brut, avant que quiconque ait dit
    /// où sont le code et les données.
    pub const ALL: Self = Self {
        read: true,
        write: true,
        execute: true,
    };
    /// Des données : lecture et écriture, jamais d'exécution.
    pub const DATA: Self = Self {
        read: true,
        write: true,
        execute: false,
    };
    /// Des données en lecture seule : `.rodata`, et le code une fois
    /// protégé.
    pub const RODATA: Self = Self {
        read: true,
        write: false,
        execute: false,
    };
    /// Du code : on le lit et on l'exécute, on ne l'écrit pas. C'est ce
    /// qui sépare un bogue de pointeur d'un programme qui se réécrit.
    pub const TEXT: Self = Self {
        read: true,
        write: false,
        execute: true,
    };

    fn allows(&self, access: Access) -> bool {
        match access {
            Access::Read => self.read,
            Access::Write => self.write,
            Access::Execute => self.execute,
        }
    }
}

/// Une zone projetée : une plage d'adresses, ses droits, et le nom sous
/// lequel l'annoncer — « .text », « la pile », « le tas ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub range: Range<u64>,
    pub perms: Perms,
    pub name: &'static str,
}

/// Mémoire simulée : un bloc contigu chargé à `base`, découpé en zones
/// qui portent chacune ses droits.
///
/// Tant qu'aucune zone n'est déclarée, tout est permis partout : c'est
/// l'état d'un bloc d'octets qu'on vient de remplir. Dès qu'un chargeur
/// dit où sont le code, les données, la pile et le tas, la mémoire se
/// comporte comme celle d'un vrai processus — on n'écrit pas dans le
/// code, on n'exécute pas les données, et ce qui n'est projeté nulle
/// part n'est pas adressable.
pub struct Memory {
    base: u64,
    data: Vec<u8>,
    /// Les zones projetées et leurs droits.
    ///
    /// Vide, la mémoire laisse tout faire : c'est l'état d'un bloc brut
    /// qu'on vient de remplir, et de tous les tests qui n'ont rien à dire
    /// des droits. Dès qu'une zone est déclarée, ce qui est hors de
    /// toutes les zones n'est plus adressable — comme dans un vrai
    /// processus, où la mémoire est trouée.
    regions: Vec<Region>,
}

impl Memory {
    pub fn new(base: u64, size: usize) -> Self {
        Self {
            base,
            data: vec![0u8; size],
            regions: Vec::new(),
        }
    }

    /// Déclare une zone et ses droits.
    ///
    /// Le nom identifie la zone : la redéclarer la remplace. C'est ce qui
    /// permet au tas de grandir — `brk` rappelle `map` avec une plage
    /// plus large — sans laisser derrière lui une pile de zones périmées.
    ///
    /// Deux zones qui se recouvrent sont départagées par l'ordre : la
    /// dernière déclarée gagne, ce qui permet de poser une pile au milieu
    /// d'un bloc déjà décrit.
    pub fn map(&mut self, range: Range<u64>, perms: Perms, name: &'static str) {
        self.regions.retain(|region| region.name != name);
        self.regions.push(Region { range, perms, name });
    }

    /// Des zones ont-elles été déclarées ? Si non, tout est permis
    /// partout, et personne ne doit changer cet état de fait sans le
    /// vouloir : déclarer une seule zone rend tout le reste inadressable.
    pub fn has_regions(&self) -> bool {
        !self.regions.is_empty()
    }

    /// Oublie toutes les zones : la mémoire redevient un bloc sans
    /// droits, où tout est permis.
    pub fn unmap_all(&mut self) {
        self.regions.clear();
    }

    /// La zone qui contient cette adresse, s'il y en a une. La dernière
    /// déclarée gagne.
    fn region_at(&self, addr: u64) -> Option<&Region> {
        self.regions
            .iter()
            .rev()
            .find(|region| region.range.contains(&addr))
    }

    /// Les droits à cette adresse, ou `None` si rien n'y est projeté.
    pub fn perms_at(&self, addr: u64) -> Option<Perms> {
        if self.regions.is_empty() {
            return Some(Perms::ALL);
        }
        self.region_at(addr).map(|region| region.perms)
    }

    /// Toutes les zones projetées, dans l'ordre des adresses.
    ///
    /// C'est la carte du processus simulé : ce que `vmmap` montre d'un
    /// vrai. Un débutant qui voit « refusé » sans savoir *où* commence
    /// le code et où finit la pile n'apprend qu'une chose : que la
    /// machine dit non.
    ///
    /// Les recouvrements sont résolus comme à la lecture — la dernière
    /// zone déclarée gagne — pour que la carte dise ce que la mémoire
    /// fera vraiment.
    pub fn regions(&self) -> Vec<Region> {
        let mut vues: Vec<Region> = Vec::new();
        for region in self.regions.iter().rev() {
            if vues.iter().any(|vue| vue.name == region.name) {
                continue;
            }
            vues.push(region.clone());
        }
        vues.sort_by_key(|region| region.range.start);
        vues
    }

    /// Le nom de la zone qui contient cette adresse — pour le dire à
    /// l'utilisateur plutôt que de lui donner un nombre.
    pub fn region_name(&self, addr: u64) -> Option<&'static str> {
        self.region_at(addr).map(|region| region.name)
    }

    /// Vérifie qu'un accès est permis sur toute la plage demandée.
    ///
    /// Sur toute la plage : un `mov qword` à cheval sur la fin du code et
    /// le début des données doit être refusé, et l'être en nommant
    /// l'octet fautif.
    pub fn check(&self, addr: u64, len: usize, access: Access) -> Result<(), MemError> {
        if self.regions.is_empty() {
            return Ok(());
        }
        for offset in 0..len.max(1) as u64 {
            let at = addr.wrapping_add(offset);
            match self.region_at(at) {
                None => return Err(MemError::Unmapped { addr: at }),
                Some(region) if !region.perms.allows(access) => {
                    return Err(MemError::Protection {
                        addr: at,
                        needed: access,
                        region: region.name,
                    });
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    pub fn base(&self) -> u64 {
        self.base
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn range(&self) -> Range<u64> {
        self.base..self.base + self.data.len() as u64
    }

    fn offset(&self, addr: u64, size: usize) -> Result<usize, MemError> {
        let off = addr
            .checked_sub(self.base)
            .ok_or(MemError::OutOfBounds { addr })?;
        let end = off
            .checked_add(size as u64)
            .ok_or(MemError::OutOfBounds { addr })?;
        if end > self.data.len() as u64 {
            return Err(MemError::OutOfBounds { addr });
        }
        Ok(off as usize)
    }

    pub fn load(&mut self, addr: u64, bytes: &[u8]) -> Result<(), MemError> {
        self.poke(addr, bytes)
    }

    /// Écrit un bloc d'octets, bornes vérifiées. Même chose que `load`,
    /// sous le nom qui convient quand ce n'est pas un programme qu'on
    /// charge mais une donnée qu'on dépose — ce que fait `read` en
    /// remplissant le tampon d'un programme.
    pub fn write_bytes(&mut self, addr: u64, bytes: &[u8]) -> Result<(), MemError> {
        self.check(addr, bytes.len(), Access::Write)?;
        self.poke(addr, bytes)
    }

    /// Écrit sans regarder les droits — ce que fait le chargeur en
    /// posant le programme, et ce que ferait un débogueur qui corrige une
    /// valeur. Le programme simulé, lui, passe par `write_bytes`.
    pub fn poke(&mut self, addr: u64, bytes: &[u8]) -> Result<(), MemError> {
        let off = self.offset(addr, bytes.len())?;
        self.data[off..off + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    pub fn bytes_at(&self, addr: u64, len: usize) -> Result<&[u8], MemError> {
        self.check(addr, len, Access::Read)?;
        self.peek_bytes(addr, len)
    }

    /// Lit sans regarder les droits : c'est ce que fait l'inspecteur —
    /// vidage mémoire, points de surveillance, recherche de chaînes.
    ///
    /// Un débogueur voit tout ; le programme, lui, obéit aux règles. Les
    /// confondre reviendrait à faire échouer l'affichage d'une zone
    /// protégée, alors que la regarder est précisément ce qu'on veut.
    pub fn peek_bytes(&self, addr: u64, len: usize) -> Result<&[u8], MemError> {
        let off = self.offset(addr, len)?;
        Ok(&self.data[off..off + len])
    }

    /// Un octet, sans contrôle de droits.
    pub fn peek_u8(&self, addr: u64) -> Result<u8, MemError> {
        Ok(self.peek_bytes(addr, 1)?[0])
    }

    pub fn read_u8(&self, addr: u64) -> Result<u8, MemError> {
        Ok(self.bytes_at(addr, 1)?[0])
    }

    pub fn read_u16(&self, addr: u64) -> Result<u16, MemError> {
        let b = self.bytes_at(addr, 2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn read_u32(&self, addr: u64) -> Result<u32, MemError> {
        let b = self.bytes_at(addr, 4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_u64(&self, addr: u64) -> Result<u64, MemError> {
        let b = self.bytes_at(addr, 8)?;
        Ok(u64::from_le_bytes(b.try_into().unwrap()))
    }

    pub fn write_u8(&mut self, addr: u64, v: u8) -> Result<(), MemError> {
        self.write_bytes(addr, &v.to_le_bytes())
    }

    pub fn write_u16(&mut self, addr: u64, v: u16) -> Result<(), MemError> {
        self.write_bytes(addr, &v.to_le_bytes())
    }

    pub fn write_u32(&mut self, addr: u64, v: u32) -> Result<(), MemError> {
        self.write_bytes(addr, &v.to_le_bytes())
    }

    pub fn write_u64(&mut self, addr: u64, v: u64) -> Result<(), MemError> {
        self.write_bytes(addr, &v.to_le_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_round_trip() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.write_u64(0x400010, 0x0102_0304_0506_0708).unwrap();
        assert_eq!(mem.read_u64(0x400010).unwrap(), 0x0102_0304_0506_0708);
        // vérifie l'ordre little-endian des octets
        assert_eq!(mem.read_u8(0x400010).unwrap(), 0x08);
    }

    #[test]
    fn load_places_bytes_at_base() {
        let mut mem = Memory::new(0x400000, 0x10);
        mem.load(0x400000, &[0x48, 0xC7, 0xC0]).unwrap();
        assert_eq!(mem.read_u8(0x400000).unwrap(), 0x48);
        assert_eq!(mem.read_u8(0x400002).unwrap(), 0xC0);
    }

    #[test]
    fn read_before_base_is_out_of_bounds() {
        let mem = Memory::new(0x400000, 0x10);
        assert_eq!(
            mem.read_u8(0x3FFFFF),
            Err(MemError::OutOfBounds { addr: 0x3FFFFF })
        );
    }

    #[test]
    fn read_past_end_is_out_of_bounds() {
        let mem = Memory::new(0x400000, 0x10);
        assert_eq!(
            mem.read_u32(0x40000E), // déborderait de 2 octets hors du bloc de 0x10
            Err(MemError::OutOfBounds { addr: 0x40000E })
        );
    }

    /// Sans zone déclarée, la mémoire est un bloc d'octets : tout est
    /// permis. C'est l'état d'un test qui n'a rien à dire des droits.
    #[test]
    fn a_bare_block_allows_everything() {
        let mut mem = Memory::new(0x400000, 0x100);
        assert_eq!(mem.perms_at(0x400000), Some(Perms::ALL));
        assert!(mem.write_bytes(0x400000, &[1, 2, 3]).is_ok());
        assert!(mem.bytes_at(0x400000, 3).is_ok());
        assert!(mem.check(0x400000, 1, Access::Execute).is_ok());
    }

    /// Une fois les zones déclarées : on n'écrit pas dans le code, on
    /// n'exécute pas les données, et le reste n'existe pas.
    #[test]
    fn mapped_memory_behaves_like_a_process() {
        let mut mem = Memory::new(0x400000, 0x4000);
        mem.map(0x400000..0x401000, Perms::TEXT, ".text");
        mem.map(0x401000..0x402000, Perms::DATA, ".data");

        // Le code s'exécute et se lit, mais ne s'écrit pas.
        assert!(mem.check(0x400000, 1, Access::Execute).is_ok());
        assert!(mem.bytes_at(0x400000, 4).is_ok());
        assert_eq!(
            mem.write_bytes(0x400004, &[0x90]),
            Err(MemError::Protection {
                addr: 0x400004,
                needed: Access::Write,
                region: ".text",
            })
        );

        // Les données s'écrivent, mais ne s'exécutent pas.
        assert!(mem.write_bytes(0x401000, &[1, 2, 3, 4]).is_ok());
        assert_eq!(
            mem.check(0x401000, 1, Access::Execute),
            Err(MemError::Protection {
                addr: 0x401000,
                needed: Access::Execute,
                region: ".data",
            })
        );

        // Et entre les deux zones déclarées, plus loin, il n'y a rien.
        assert_eq!(
            mem.write_bytes(0x403000, &[0]),
            Err(MemError::Unmapped { addr: 0x403000 })
        );
    }

    /// Un accès à cheval sur deux zones est refusé sur l'octet fautif, et
    /// non sur son premier octet : c'est celui-là qu'il faut montrer.
    #[test]
    fn an_access_that_straddles_two_regions_names_the_offending_byte() {
        let mut mem = Memory::new(0x400000, 0x4000);
        mem.map(0x400000..0x401000, Perms::DATA, ".data");
        mem.map(0x401000..0x402000, Perms::RODATA, ".rodata");
        assert_eq!(
            mem.write_bytes(0x400ffc, &[0; 8]),
            Err(MemError::Protection {
                addr: 0x401000,
                needed: Access::Write,
                region: ".rodata",
            })
        );
    }

    /// Une écriture de huit octets obéit aux droits comme une écriture
    /// d'un seul. Elles passaient toutes par le chemin du *chargeur*, qui
    /// n'en vérifie aucun : la protection existait et ne protégeait rien.
    #[test]
    fn a_sized_write_obeys_the_permissions_too() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.map(0x400000..0x401000, Perms::TEXT, ".text");
        for résultat in [
            mem.write_u8(0x400000, 1),
            mem.write_u16(0x400000, 1),
            mem.write_u32(0x400000, 1),
            mem.write_u64(0x400000, 1),
        ] {
            assert!(
                matches!(résultat, Err(MemError::Protection { .. })),
                "{résultat:?}"
            );
        }
    }

    /// L'inspecteur voit tout : un vidage mémoire doit pouvoir montrer
    /// une zone que le programme n'a pas le droit de lire, et le chargeur
    /// doit pouvoir déposer le programme dans du code non inscriptible.
    #[test]
    fn the_debugger_sees_what_the_program_may_not_touch() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.map(0x400000..0x401000, Perms::TEXT, ".text");
        assert!(
            mem.load(0x400000, &[0xB8, 0x01]).is_ok(),
            "le chargeur pose"
        );
        assert_eq!(mem.peek_u8(0x400000), Ok(0xB8));
        assert!(mem.peek_bytes(0x400000, 2).is_ok());
        // Mais le programme, lui, ne peut pas y écrire.
        assert!(mem.write_bytes(0x400000, &[0x90]).is_err());
    }
}
