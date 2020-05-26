use crate::cpu::cpu::AddressingMode;
use crate::cpu::opscode;
use byteorder::{ByteOrder, LittleEndian};
use std::cmp::min;
use std::collections::HashMap;

pub struct Disasm {
    pub program: Vec<String>,
    pub hex_dump: Vec<Vec<u8>>,
    pub ops_index_map: HashMap<u16, usize>,
}

impl Disasm {
    pub fn new(program: &[u8], start: usize) -> Self {
        let ref opscodes: HashMap<u8, &'static opscode::OpsCode> = *opscode::OPSCODES_MAP;

        let mut begin = start;
        let mut asm = Vec::new();
        let mut mapping: HashMap<u16, usize> = HashMap::new();
        let mut hex_dump: Vec<Vec<u8>> = Vec::new();
        while begin < program.len() {
            //todo: should be another condition as well
            let code = &program[begin];
            if !opscodes.contains_key(code) {
                panic!("unknown ops code {:02x}", code);
            }

            let ops = opscodes.get(code).unwrap();

            let tmp = match ops.len {
                1 => {
                    hex_dump.push(vec![*code]);
                    String::from("")
                }
                2 => {
                    let address: u8 = program[begin + 1];
                    hex_dump.push(vec![*code, address]);
                    match ops.mode {
                        AddressingMode::Immediate => format!("#${:02x}", address),
                        AddressingMode::ZeroPage => format!("${:02x}", address),
                        AddressingMode::ZeroPage_X => format!("${:02x},X", address),
                        AddressingMode::ZeroPage_Y => format!("${:02x},Y", address),
                        AddressingMode::Indirect_X => format!("(${:02x},X)", address),
                        AddressingMode::Indirect_Y => format!("(${:02x}),Y", address),
                        AddressingMode::NoneAddressing => {
                            // assuming local jumps: BNE, BVS, etc.... todo: check ?
                            let address: usize = (begin + 2).wrapping_add((address as i8) as usize);
                            format!("${:04x}", address)
                        }

                        _ => panic!(
                            "unexpected addressing mode {:?} has ops-len 2. code {:02x}",
                            ops.mode, ops.code
                        ),
                    }
                }
                3 => {
                    if begin + 1 >= program.len() || begin + 2 >= program.len() {
                        panic!("unexpected end of program. code {:02x} requires 2 parameters, but only {} byte(s) left ", ops.code, program.len() - begin);
                    }
                    hex_dump.push(vec![*code, program[begin + 1], program[begin + 2]]);
                    format!(
                        "${:04x}",
                        LittleEndian::read_u16(&program[begin + 1 as usize..])
                    )
                }
                _ => String::from(""),
            };

            let asm_str = format!("{:04x}: {} {}", begin, ops.mnemonic, tmp)
                .trim()
                .to_string();

            asm.push(asm_str);
            mapping.insert(begin as u16, asm.len() - 1);
            begin += ops.len as usize;
        }
        Disasm {
            program: asm,
            ops_index_map: mapping,
            hex_dump: hex_dump,
        }
    }

    pub fn slice(&self, pos: u16) -> (&[String], usize) {
        let index = *self.ops_index_map.get(&pos).unwrap();
        let slice_size = min(10 as usize, self.program.len());

        if index > slice_size / 2 {
            let end = min(self.program.len(), index + slice_size / 2);
            let begin = end - slice_size;
            (&self.program[begin..end], index - begin)
        } else {
            (&self.program[0..slice_size], index)
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
    use pretty_assertions::assert_eq;

    #[test]
    fn test() {
        let asm = Disasm::new(&CPU::transform("a2 08 ca"), 0);
        let result = vec!["0000: LDX #$08", "0002: DEX"];
        assert_eq!(asm.program, result);
        assert_eq!(asm.hex_dump, vec!(vec!(0xa2, 0x08), vec!(0xca)));
        assert_eq!(asm.ops_index_map.get(&0), Some(&0));
        assert_eq!(asm.ops_index_map.get(&2), Some(&1));
    }

    #[test]
    fn test_slice() {
        let asm = Disasm::new(
            &CPU::transform("a2 08 ca c8 e0 03 d0 fa 00 a2 08 ca c8 e0 03 d0 fa 00"),
            0,
        );
        let result = vec![
            "0000: LDX #$08",
            "0002: DEX",
            "0003: INY",
            "0004: CPX #$03",
            "0006: BNE $0002",
            "0008: BRK",
            "0009: LDX #$08",
            "000b: DEX",
            "000c: INY",
            "000d: CPX #$03",
            "000f: BNE $000b",
            "0011: BRK",
        ];
        assert_eq!(asm.program, result);
        let (slice, idx) = asm.slice(0004);
        assert_eq!(3, idx);
        assert_eq!(10, slice.len());
        assert_eq!(slice, &result[0..10]);
    }

    #[test]
    fn test_slice_end_of_program() {
        let asm = Disasm::new(
            &CPU::transform("a2 08 ca c8 e0 03 d0 fa 00 a2 08 ca c8 e0 03 d0 fa 00"),
            0,
        );
        let result = vec![
            "0000: LDX #$08",
            "0002: DEX",
            "0003: INY",
            "0004: CPX #$03",
            "0006: BNE $0002",
            "0008: BRK",
            "0009: LDX #$08",
            "000b: DEX",
            "000c: INY",
            "000d: CPX #$03",
            "000f: BNE $000b",
            "0011: BRK",
        ];
        assert_eq!(asm.program, result);
        let (slice, idx) = asm.slice(0x000f);
        assert_eq!(8, idx);
        assert_eq!(10, slice.len());
        assert_eq!(slice, &result[2..12]);
    }
}
