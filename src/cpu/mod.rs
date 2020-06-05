use crate::cpu::mem::AddressingMode;
use cpu::CPU;
use std::collections::HashMap;

pub mod cpu;
pub mod mem;
pub mod opscode;

pub fn trace(cpu: &CPU) -> String {
    let ref opscodes: HashMap<u8, &'static opscode::OpsCode> = *opscode::OPSCODES_MAP;
    let code = cpu.mem_read(cpu.program_counter);
    let ops = opscodes.get(&code).unwrap();

    let begin = cpu.program_counter;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let tmp = match ops.len {
        1 => {
            match ops.mode {
                AddressingMode::Accumulator => format!("A "),
                _ => String::from(""),
            }
        }
        2 => {
            let address: u8 = cpu.mem_read(begin + 1);
            // let value = cpu.mem_read(address));
            hex_dump.push(address);
            match ops.mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => {
                    let (mem_addr, stored_value) = ops.mode.read_u8_from_pos(cpu, address as u16);
                    format!("${:02x} = {:02x}", mem_addr, stored_value)
                }
                AddressingMode::ZeroPage_X => {
                    let (mem_addr, stored_value) = ops.mode.read_u8_from_pos(cpu, address as u16);
                    format!("${:02x},X @ {:02x} = {:02x}", address, mem_addr, stored_value)
                }
                AddressingMode::ZeroPage_Y => {
                    let (mem_addr, stored_value) = ops.mode.read_u8_from_pos(cpu, address as u16);
                    format!("${:02x},Y @ {:02x} = {:02x}", address, mem_addr, stored_value)
                }
                AddressingMode::Indirect_X => {
                    let (mem_addr, stored_value) = ops.mode.read_u8_from_pos(cpu, address as u16);
                    format!("(${:02x},X) @ {:04x} = {:02x}", address, mem_addr, stored_value)
                }
                AddressingMode::Indirect_Y => {
                    let (mem_addr, stored_value) = ops.mode.read_u8_from_pos(cpu, address as u16);
                    format!("(${:02x}),Y @ {:04x} = {:02x}", address, mem_addr, stored_value)
                }
                AddressingMode::NoneAddressing => {
                    // assuming local jumps: BNE, BVS, etc.... todo: check ?
                    let address: usize =
                        (begin as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04x}", address)
                }

                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 2. code {:02x}",
                    ops.mode, ops.code
                ),
            }
        }
        3 => {
            let address_lo = cpu.mem_read(begin + 1);
            let address_hi = cpu.mem_read(begin + 2);
            hex_dump.push(address_lo);
            hex_dump.push(address_hi);

            let address = cpu.mem_read_u16(begin + 1);


            match ops.mode {
                AddressingMode::NoneAddressing => {
                    if ops.code == 0x6c { //jmp indirect
                        let jmp_addr = if address & 0x00FF == 0x00FF {
                            let lo = cpu.mem_read(address);
                            let hi = cpu.mem_read(address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.mem_read_u16(address)
                        };
        
                        // let jmp_addr = cpu.mem_read_u16(address);
                        format!("(${:04x}) = {:04x}", address, jmp_addr)
                    } else {
                        format!("${:04x}", address)
                    }
                }
                AddressingMode::Absolute => {
                    let (mem_addr, stored_value) = ops.mode.read_u8_from_pos(cpu, address);

                    format!("${:04x} = {:02x}", mem_addr, stored_value)

                }
                AddressingMode::Absolute_X => {
                    let (mem_addr, stored_value) = ops.mode.read_u8_from_pos(cpu, address);

                    format!("${:04x},X @ {:04x} = {:02x}", address, mem_addr, stored_value)
                }
                AddressingMode::Absolute_Y => {
                    let (mem_addr, stored_value) = ops.mode.read_u8_from_pos(cpu, address);
                    format!("${:04x},Y @ {:04x} = {:02x}", address, mem_addr, stored_value)
                }
                _ => panic!("unexpected addressing mode {:?} has ops-len 3. code {:02x}", ops.mode, ops.code)
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!("{:04x}  {:8}  {} {}", begin, hex_str, ops.mnemonic, tmp)
        .trim()
        .to_string();

    format!(
        // "{:47} A:{:02x} X:{:02x} Y:{:02x} SP:{:02x} FL:{:08b}",
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
        // "{:30}(a:{:x}, x:{:x}, y:{:x}, sp:{:x}, fl:{:x})",
        asm_str, cpu.register_a, cpu.register_x, cpu.register_y, cpu.flags, cpu.stack_pointer
    ).to_ascii_uppercase()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::bus::bus::MockBus;

    #[test]
    fn test_format_trace() {
        let mut mem = MockBus::new();
        mem.space[100] = 0xa2;
        mem.space[101] = 0x01;
        mem.space[102] = 0xca;
        mem.space[103] = 0x88;
        let mut cpu = CPU::new(&mut mem);
        cpu.program_counter = 0x64;
        cpu.register_a = 1;
        cpu.register_x = 2;
        cpu.register_y = 3;
        let mut result: Vec<String> = vec![];
        cpu.interpret_fn(0x64 + 4, |cpu| {
            result.push(trace(&cpu));
        });
        assert_eq!(
            "0064  A2 01     LDX #$01                        A:01 X:02 Y:03 P:24 SP:FD",
            result[0]
        );
        assert_eq!(
            "0066  CA        DEX                             A:01 X:01 Y:03 P:24 SP:FD",
            result[1]
        );
        assert_eq!(
            "0067  88        DEY                             A:01 X:00 Y:03 P:26 SP:FD",
            result[2]
        ); //zero flag
    }

    #[test]
    fn test_format_mem_access() {
        let mut mem = MockBus::new();
        // ORA ($33), Y
        mem.space[100] = 0x11;
        mem.space[101] = 0x33;

        //data
        mem.space[0x33] = 00;
        mem.space[0x34] = 04;
        mem.space[0x400] = 0xAA;
        let mut cpu = CPU::new(&mut mem);
        cpu.program_counter = 0x64;
        cpu.register_y = 0;
        let mut result: Vec<String> = vec![];
        cpu.interpret_fn(0x64 + 2, |cpu| {
            result.push(trace(&cpu));
        });
        assert_eq!(
            "0064  11 33     ORA ($33),Y @ 0400 = AA         A:00 X:00 Y:00 P:24 SP:FD",
            result[0]
        );
    }
}
