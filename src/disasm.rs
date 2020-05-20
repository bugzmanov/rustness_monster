use crate::cpu::opscode;
use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashMap;
use std::cmp::min;

pub struct Disasm {
    program: Vec<String>,
    ops_index_map: HashMap<u16, usize>,
}


impl Disasm {

    pub fn new(program: &[u8], start: usize) -> Self {
        let ref opscodes: HashMap<u8, &'static opscode::OpsCode> = *opscode::OPSCODES_MAP;
    
        let mut begin = start;
        let mut asm = Vec::new();
        let mut mapping: HashMap<u16, usize> = HashMap::new();
        while begin < program.len() {
            let code = &program[begin];
            let ops = opscodes.get(code).unwrap();
    
            let tmp = match ops.len {
                2 => format!("#${:02x}", program[begin + 1]),
                3 => format!(
                    "#{:x}",
                    LittleEndian::read_u16(&program[begin + 1 as usize..])
                ),
                _ => String::from(""),
            };
    
            asm.push(format!("{:04x}: {} {}", begin, ops.mnemonic, tmp).trim().to_string());
            mapping.insert(begin as u16, asm.len()-1);
            begin += ops.len as usize;
        }
        Disasm {
            program: asm,
            ops_index_map: mapping,
        }    
    }

    pub fn slice(&self, pos: u16) -> (&[String], usize) {
        let index = *self.ops_index_map.get(&pos).unwrap();
        let slice_size = min(10 as usize, self.program.len());
        
        if index > slice_size / 2 { 
            let end = min(self.program.len(), index + slice_size/2);
            let begin = end - slice_size;
            (&self.program[begin .. end], index - begin)
        } else {
            (&self.program[0 .. slice_size], index) 
        }
    }
}
pub fn disasm(program: &[u8], start: usize) -> Vec<String> {
    let ref opscodes: HashMap<u8, &'static opscode::OpsCode> = *opscode::OPSCODES_MAP;

    let mut begin = start;
    let mut result = Vec::new();
    while begin < program.len() {
        let code = &program[begin];
        let ops = opscodes.get(code).unwrap();

        let tmp = match ops.len {
            2 => format!("#${:02x}", program[begin + 1]),
            3 => format!(
                "#{:x}",
                LittleEndian::read_u16(&program[begin + 1 as usize..])
            ),
            _ => String::from(""),
        };

        result.push(format!("{:04x}: {} {}", begin, ops.mnemonic, tmp));
        begin += ops.len as usize;
    }
    result    
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::cpu::cpu::CPU;

    #[test]
    fn test() {
        let asm = Disasm::new(&CPU::transform("a2 08 ca"), 0);
        let result= vec!("0000: LDX #$08", "0002: DEX");
        assert_eq!(asm.program, result);
        assert_eq!(asm.ops_index_map.get(&0), Some(&0));
        assert_eq!(asm.ops_index_map.get(&2), Some(&1));
    }

    #[test]
    fn test_slice() {
        let asm = Disasm::new(&CPU::transform("a2 08 ca c8 e0 03 d0 fa 00 a2 08 ca c8 e0 03 d0 fa 00"), 0);
        let result= vec!("0000: LDX #$08", "0002: DEX", "0003: INY", "0004: CPX #$03", "0006: BNE #$fa", "0008: BRK", "0009: LDX #$08", "000b: DEX", "000c: INY", "000d: CPX #$03", "000f: BNE #$fa", "0011: BRK");
        assert_eq!(asm.program, result);
        let (slice, idx) = asm.slice(0004);
        assert_eq!(3, idx);
        assert_eq!(10, slice.len());
        assert_eq!(slice, &["0000: LDX #$08", "0002: DEX", "0003: INY", "0004: CPX #$03", "0006: BNE #$fa", "0008: BRK", "0009: LDX #$08", "000b: DEX", "000c: INY","000d: CPX #$03"]);
    }

    #[test]
    fn test_slice_end_of_program() {
        let asm = Disasm::new(&CPU::transform("a2 08 ca c8 e0 03 d0 fa 00 a2 08 ca c8 e0 03 d0 fa 00"), 0);
        let result= vec!("0000: LDX #$08", "0002: DEX", "0003: INY", "0004: CPX #$03", "0006: BNE #$fa", "0008: BRK", "0009: LDX #$08", "000b: DEX", "000c: INY", "000d: CPX #$03", "000f: BNE #$fa", "0011: BRK");
        assert_eq!(asm.program, result);
        let (slice, idx) = asm.slice(0x000f);
        assert_eq!(8, idx);
        assert_eq!(10, slice.len());
        assert_eq!(slice, &["0003: INY", "0004: CPX #$03", "0006: BNE #$fa", "0008: BRK", "0009: LDX #$08", "000b: DEX", "000c: INY", "000d: CPX #$03", "000f: BNE #$fa", "0011: BRK"]);
    }
}