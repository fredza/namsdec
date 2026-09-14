//! Désassemblage x86-64 via `iced-x86`, formaté en syntaxe NASM — cohérent
//! avec le reste de l'outil. Réutilisé tel quel en phase 2 pour le futur
//! désassembleur ELF (seule la source des octets change).

use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};

#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    pub address: u64,
    pub bytes: Vec<u8>,
    pub text: String,
}

/// Décode une seule instruction en tête de `bytes`, positionnée à `ip`.
/// Réutilisé par `namsdec-emu` pour le fetch/decode de l'interpréteur —
/// seule la sémantique d'exécution y est réécrite à la main.
pub fn decode_one(bytes: &[u8], ip: u64) -> Option<Instruction> {
    let mut decoder = Decoder::with_ip(64, bytes, ip, DecoderOptions::NONE);
    if !decoder.can_decode() {
        return None;
    }
    let instr = decoder.decode();
    if instr.is_invalid() {
        None
    } else {
        Some(instr)
    }
}

/// Désassemble `bytes` en supposant qu'ils sont chargés à partir de
/// `base_addr` (mode 64 bits).
pub fn disassemble(bytes: &[u8], base_addr: u64) -> Vec<DecodedInstruction> {
    let mut decoder = Decoder::with_ip(64, bytes, base_addr, DecoderOptions::NONE);
    let mut formatter = NasmFormatter::new();
    let mut instr = Instruction::default();
    let mut result = Vec::new();
    let mut text = String::new();

    while decoder.can_decode() {
        decoder.decode_out(&mut instr);
        text.clear();
        formatter.format(&instr, &mut text);

        let start = (instr.ip() - base_addr) as usize;
        let end = start + instr.len();
        result.push(DecodedInstruction {
            address: instr.ip(),
            bytes: bytes[start..end].to_vec(),
            text: text.clone(),
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_mov_eax_ebx() {
        let out = disassemble(&[0x89, 0xD8], 0x400000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].address, 0x400000);
        assert_eq!(out[0].bytes, vec![0x89, 0xD8]);
        assert_eq!(out[0].text, "mov eax,ebx");
    }

    #[test]
    fn decodes_sum_loop_program() {
        // mov eax,0 / mov ecx,5 / add rax,rcx / dec rcx / jnz $-6 / hlt
        // (exactement les octets produits par nasm pour le programme de
        // somme du plan — voir namsdec-asm::tests::assembles_sum_loop_*)
        let bytes: &[u8] = &[
            0xB8, 0x00, 0x00, 0x00, 0x00, // mov eax,0
            0xB9, 0x05, 0x00, 0x00, 0x00, // mov ecx,5
            0x48, 0x01, 0xC8, // add rax,rcx
            0x48, 0xFF, 0xC9, // dec rcx
            0x75, 0xF8, // jnz sum_loop
            0xF4, // hlt
        ];
        let out = disassemble(bytes, 0x400000);
        assert_eq!(out.len(), 6);
        assert_eq!(out[0].address, 0x400000);
        assert_eq!(out[2].address, 0x40000A); // add rax,rcx
        assert_eq!(out[2].text, "add rax,rcx");
        assert_eq!(out[5].address, 0x400012); // hlt
        assert_eq!(out[5].text, "hlt");
    }
}
