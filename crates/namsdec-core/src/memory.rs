use std::ops::Range;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq, Clone, Copy)]
pub enum MemError {
    #[error("accès mémoire hors bornes à l'adresse {addr:#x}")]
    OutOfBounds { addr: u64 },
}

/// Mémoire simulée : un unique bloc contigu chargé à `base`. Suffisant pour
/// la phase 1 (format `nasm -f bin`) ; remplaçable en phase 2 par une
/// mémoire segmentée (r/w/x) sans changer cette API.
pub struct Memory {
    base: u64,
    data: Vec<u8>,
}

impl Memory {
    pub fn new(base: u64, size: usize) -> Self {
        Self {
            base,
            data: vec![0u8; size],
        }
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
        let off = self.offset(addr, bytes.len())?;
        self.data[off..off + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    pub fn bytes_at(&self, addr: u64, len: usize) -> Result<&[u8], MemError> {
        let off = self.offset(addr, len)?;
        Ok(&self.data[off..off + len])
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
        self.load(addr, &v.to_le_bytes())
    }

    pub fn write_u16(&mut self, addr: u64, v: u16) -> Result<(), MemError> {
        self.load(addr, &v.to_le_bytes())
    }

    pub fn write_u32(&mut self, addr: u64, v: u32) -> Result<(), MemError> {
        self.load(addr, &v.to_le_bytes())
    }

    pub fn write_u64(&mut self, addr: u64, v: u64) -> Result<(), MemError> {
        self.load(addr, &v.to_le_bytes())
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
}
