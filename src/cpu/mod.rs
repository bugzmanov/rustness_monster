use crate::bus::bus::CpuBus;
use crate::cpu::mem::AddressingMode;
use cpu::CPU;
use std::collections::HashMap;

pub mod cpu;
pub mod mem;
pub mod opscode;

lazy_static! {
    pub static ref NON_READABLE_ADDR: Vec<u16> =
        vec!(0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x4016, 0x4017);
}

pub fn trace(cpu: &mut CPU) -> String {
    let ref opscodes: HashMap<u8, &'static opscode::OpsCode> = *opscode::OPSCODES_MAP;
    let ref non_readable_addr = *NON_READABLE_ADDR;

    let code = cpu.mem_read(cpu.program_counter);
    let ops = opscodes.get(&code).unwrap();

    let begin = cpu.program_counter;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let (mem_addr, stored_value) = match ops.mode {
        AddressingMode::Immediate
        | AddressingMode::NoneAddressing
        | AddressingMode::Accumulator => (0, 0),
        _ => {
            let address = if ops.len == 2 {
                cpu.mem_read(begin + 1) as u16
            } else {
                cpu.mem_read_u16(begin + 1)
            };
            let (_, addr) = ops.mode.get_absolute_addr(cpu, address);
            if !non_readable_addr.contains(&addr) {
                (addr, cpu.mem_read(addr))
            } else {
                (addr, 0)
            }
        }
    };

    let tmp = match ops.len {
        1 => match ops.mode {
            AddressingMode::Accumulator => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let address: u8 = cpu.mem_read(begin + 1);
            // let value = cpu.mem_read(address));
            hex_dump.push(address);

            match ops.mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPage_X => format!(
                    "${:02x},X @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::ZeroPage_Y => format!(
                    "${:02x},Y @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::Indirect_X => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    address, (address.wrapping_add(cpu.register_x)), mem_addr, stored_value
                ),
                AddressingMode::Indirect_Y | AddressingMode::Indirect_Y_PageCross => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    address, (mem_addr.wrapping_sub(cpu.register_y as u16)), mem_addr, stored_value
                ),
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
                    if ops.code == 0x6c {
                        //jmp indirect
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
                AddressingMode::Absolute => format!("${:04x} = {:02x}", mem_addr, stored_value),
                AddressingMode::Absolute_X | AddressingMode::Absolute_X_PageCross => format!(
                    "${:04x},X @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::Absolute_Y | AddressingMode::Absolute_Y_PageCross => format!(
                    "${:04x},Y @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 3. code {:02x}",
                    ops.mode, ops.code
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!("{:04x}  {:8} {: >4} {}", begin, hex_str, ops.mnemonic, tmp)
        .trim()
        .to_string();

    let bus_trace = cpu.bus.trace();
    format!(
        // "{:47} A:{:02x} X:{:02x} Y:{:02x} SP:{:02x} FL:{:08b}",
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x} PPU:{:3},{:3} CYC:{}",
        // "{:30}(a:{:x}, x:{:x}, y:{:x}, sp:{:x}, fl:{:x})",
        asm_str,
        cpu.register_a,
        cpu.register_x,
        cpu.register_y,
        cpu.flags,
        cpu.stack_pointer,
        bus_trace.ppu_cycles,
        bus_trace.ppu_scanline,
        bus_trace.cpu_cycles
    )
    .to_ascii_uppercase()
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
        let mut cpu = CPU::new(Box::from(mem));
        cpu.program_counter = 0x64;
        cpu.register_a = 1;
        cpu.register_x = 2;
        cpu.register_y = 3;
        let mut result: Vec<String> = vec![];
        cpu.interpret_fn(0x64 + 4, |cpu| {
            result.push(trace(cpu));
        });
        assert_eq!(
            "0064  A2 01     LDX #$01                        A:01 X:02 Y:03 P:24 SP:FD PPU:  0,  0 CYC:0",
            result[0]
        );
        assert_eq!(
            "0066  CA        DEX                             A:01 X:01 Y:03 P:24 SP:FD PPU:  0,  0 CYC:2",
            result[1]
        );
        assert_eq!(
            "0067  88        DEY                             A:01 X:00 Y:03 P:26 SP:FD PPU:  0,  0 CYC:4",
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
        let mut cpu = CPU::new(Box::from(mem));
        cpu.program_counter = 0x64;
        cpu.register_y = 0;
        let mut result: Vec<String> = vec![];
        cpu.interpret_fn(0x64 + 2, |cpu| {
            result.push(trace(cpu));
        });
        assert_eq!(
            "0064  11 33     ORA ($33),Y = 0400 @ 0400 = AA  A:00 X:00 Y:00 P:24 SP:FD PPU:  0,  0 CYC:0",
            result[0]
        );
    }
}
