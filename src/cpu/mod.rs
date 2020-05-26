use std::collections::HashMap;
use cpu::AddressingMode;
use cpu::CPU;
use cpu::Memory;

pub mod cpu;
pub mod opscode;
// pub mod tracer;

pub fn trace(cpu: &CPU) -> String {
    let ref opscodes: HashMap<u8, &'static opscode::OpsCode> = *opscode::OPSCODES_MAP;
    let code = cpu.mem_read(cpu.program_counter);
    let ops = opscodes.get(&code).unwrap();

    let begin = cpu.program_counter;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let tmp = match ops.len {
        1 => {
            String::from("")
        }
        2 => {
            let address: u8 = cpu.mem_read(begin + 1);
            hex_dump.push(address);
            match ops.mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => format!("${:02x}", address),
                AddressingMode::ZeroPage_X => format!("${:02x},X", address),
                AddressingMode::ZeroPage_Y => format!("${:02x},Y", address),
                AddressingMode::Indirect_X => format!("(${:02x},X)", address),
                AddressingMode::Indirect_Y => format!("(${:02x}),Y", address),
                AddressingMode::NoneAddressing => {
                    // assuming local jumps: BNE, BVS, etc.... todo: check ?
                    let address: usize = (begin as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04x}", address)
                }

                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 2. code {:02x}",
                    ops.mode, ops.code
                ),
            }
        }
        3 => {
            let address_lo = cpu.mem_read(begin+1);
            let address_hi = cpu.mem_read(begin+2);
            hex_dump.push(address_lo);
            hex_dump.push(address_hi);
            format!(
                "${:04x}",
                cpu.mem_read_u16(begin +1)
            )
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump.iter().map(|z| format!("{:02x}", z)).collect::<Vec<String>>().join(" ");
    let asm_str = format!("{:04x}: {:8} {} {}", begin, hex_str, ops.mnemonic, tmp)
        .trim()
        .to_string();


    format!("{:30}(a:{}, x:{}, y:{}, sp:{}, fl:{:08b})", asm_str, cpu.register_a, cpu.register_x, cpu.register_y, cpu.stack_pointer, cpu.flags)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_format_trace() {
        let mut mem = Memory::new();
        mem.space[100] = 0xa2;
        mem.space[101] = 0x01;
        mem.space[102] = 0xca;
        mem.space[103] = 0x88;
        let mut cpu = CPU::new(&mut mem);
        cpu.program_counter = 0x64;
        cpu.register_a=1;
        cpu.register_x=2;
        cpu.register_y=3;
        let mut result: Vec<String> = vec![];
        cpu.interpret_fn(0x64 + 4, |cpu| {
            result.push(trace(&cpu));
        });
        assert_eq!("0064: a2 01    LDX #$01       (a:1, x:2, y:3, sp:255, fl:00000000)", result[0]);
        assert_eq!("0066: ca       DEX            (a:1, x:1, y:3, sp:255, fl:00000000)", result[1]);
        assert_eq!("0067: 88       DEY            (a:1, x:0, y:3, sp:255, fl:00000010)", result[2]); //zero flag
        
    }
}