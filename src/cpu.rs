use crate::opscode;
use byteorder::{ByteOrder, LittleEndian};
use hex;
use std::collections::HashMap;

bitflags! {

/// # Status Register (P)
///
///  7 6 5 4 3 2 1 0
///  N V _ B D I Z C
///  | |   | | | | +--- Carry Flag
///  | |   | | | +----- Zero Flag
///  | |   | | +------- Interrupt Disable
///  | |   | +--------- Decimal Mode (Allows BCD, not implemented on NES)
///  | |   +----------- Break Command
///  | +--------------- Overflow Flag
///  +----------------- Negative Flag
///
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const OVERFLOW          = 0b01000000;
        const NEGATIV           = 0b10000000;
    }
}

struct Memory {
    space: [u8; 0xffff],
}

/// # Memory Map http://nesdev.com/NESDoc.pdf
///
///  _______________ $10000  _______________
/// | PRG-ROM       |       |               |
/// | Upper Bank    |       |               |
/// |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
/// | PRG-ROM       |       |               |
/// | Lower Bank    |       |               |
/// |_______________| $8000 |_______________|
/// | SRAM          |       | SRAM          |
/// |_______________| $6000 |_______________|
/// | Expansion ROM |       | Expansion ROM |
/// |_______________| $4020 |_______________|
/// | I/O Registers |       |               |
/// |_ _ _ _ _ _ _ _| $4000 |               |
/// | Mirrors       |       | I/O Registers |
/// | $2000-$2007   |       |               |
/// |_ _ _ _ _ _ _ _| $2008 |               |
/// | I/O Registers |       |               |
/// |_______________| $2000 |_______________|
/// | Mirrors       |       |               |
/// | $0000-$07FF   |       |               |
/// |_ _ _ _ _ _ _ _| $0800 |               |
/// | RAM           |       | RAM           |
/// |_ _ _ _ _ _ _ _| $0200 |               |
/// | Stack         |       |               |
/// |_ _ _ _ _ _ _ _| $0100 |               |
/// | Zero Page     |       |               |
/// |_______________| $0000 |_______________|
///
trait Mem {
    const ZERO_PAGE: u16 = 0x0;
    const STACK: u16 = 0x0100;
    const RAM: u16 = 0x0200;
    const RAM_MIRRORS: u16 = 0x0800;
    const IO_REGISTERS: u16 = 0x2000;
    const IO_MIRRORS: u16 = 0x2008;

    fn write(&mut self, pos: u16, data: u8);
    fn read(&self, pos: u16) -> u8;
    fn read_u16(&self, pos: u16) -> u16;
}

impl Mem for Memory {
    fn write(&mut self, pos: u16, data: u8) {
        self.space[pos as usize] = data
    }

    fn read(&self, pos: u16) -> u8 {
        self.space[pos as usize]
    }

    fn read_u16(&self, pos: u16) -> u16 {
        LittleEndian::read_u16(&self.space[pos as usize..])
    }
}

impl Memory {
    pub fn new() -> Self {
        Memory { space: [0; 0xFFFF] }
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    Accumulator,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    // Indirect,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

impl AddressingMode {
    pub fn read_u8(&self, mem: &[u8], cpu: &CPU) -> u8 {
        if let AddressingMode::Accumulator = self {
            return cpu.register_a;
        }

        let pos: u8 = mem[cpu.program_counter as usize];
        match self {
            AddressingMode::Immediate => pos,
            AddressingMode::ZeroPage => cpu.memory.read(pos as u16),
            AddressingMode::ZeroPage_X => cpu.memory.read((pos + cpu.register_x) as u16),
            AddressingMode::ZeroPage_Y => cpu.memory.read((pos + cpu.register_y) as u16),
            AddressingMode::Absolute => {
                let mem_address = LittleEndian::read_u16(&mem[cpu.program_counter as usize..]);
                cpu.memory.read(mem_address)
            }
            AddressingMode::Absolute_X => {
                let mem_address = LittleEndian::read_u16(&mem[cpu.program_counter as usize..])
                    + cpu.register_x as u16;
                cpu.memory.read(mem_address)
            }
            AddressingMode::Absolute_Y => {
                let mem_address = LittleEndian::read_u16(&mem[cpu.program_counter as usize..])
                    + cpu.register_y as u16;
                cpu.memory.read(mem_address)
            }

            AddressingMode::Indirect_X => {
                let ptr: u8 = pos + cpu.register_x; //todo overflow
                let deref = cpu.memory.read_u16(ptr as u16);
                cpu.memory.read(deref)
            }
            AddressingMode::Indirect_Y => {
                let deref = cpu.memory.read_u16(pos as u16) + cpu.register_y as u16;
                cpu.memory.read(deref)
            }
            AddressingMode::Accumulator => panic!("should not reach this code"),
            AddressingMode::NoneAddressing => {
                panic!("AddressingMode::NoneAddressing shouldn't be used to read data")
            }
        }
    }

    pub fn write_u8(&self, mem: &[u8], cpu: &mut CPU, data: u8) {
        if let AddressingMode::Accumulator = self {
            cpu.set_register_a(data);
            return;
        }

        let pos: u8 = mem[cpu.program_counter as usize];

        match self {
            AddressingMode::Immediate => panic!("Immediate adressing mode only for reading"),
            AddressingMode::ZeroPage => cpu.memory.write(pos as u16, data),
            AddressingMode::ZeroPage_X => cpu.memory.write((pos + cpu.register_x) as u16, data),
            AddressingMode::ZeroPage_Y => cpu.memory.write((pos + cpu.register_y) as u16, data),
            AddressingMode::Absolute => {
                let mem_address = LittleEndian::read_u16(&mem[cpu.program_counter as usize..]);
                cpu.memory.write(mem_address, data)
            }
            AddressingMode::Absolute_X => {
                let mem_address = LittleEndian::read_u16(&mem[cpu.program_counter as usize..])
                    + cpu.register_x as u16;
                cpu.memory.write(mem_address, data)
            }
            AddressingMode::Absolute_Y => {
                let mem_address = LittleEndian::read_u16(&mem[cpu.program_counter as usize..])
                    + cpu.register_y as u16;
                cpu.memory.write(mem_address, data)
            }
            AddressingMode::Indirect_X => {
                let ptr: u8 = pos + cpu.register_x; //todo overflow
                let deref = cpu.memory.read_u16(ptr as u16);
                cpu.memory.write(deref, data)
            }
            AddressingMode::Indirect_Y => {
                let deref = cpu.memory.read_u16(pos as u16) + cpu.register_y as u16;
                cpu.memory.write(deref, data)
            }
            AddressingMode::Accumulator => {
                panic!("shouldn't be here");
                // cpu.set_register_a(data)
            }
            AddressingMode::NoneAddressing => {
                panic!("AddressingMode::NoneAddressing shouldn't be used to read data")
            }
        }
    }
}

pub struct CPU {
    register_a: u8,
    register_x: u8,
    register_y: u8,
    stack_pointer: u8,
    program_counter: u16,
    flags: CpuFlags,
    memory: Memory,
}

impl CPU {
    pub fn transform(s: &str) -> Vec<u8> {
        hex::decode(s.replace(' ', "")).expect("Decoding failed")
    }

    fn add_to_register_a(&mut self, data: u8) {
        let sum = data.wrapping_add(self.register_a);
        // if let None = data.checked_add(self.register_a) { //todo: why is this not working?
        let register_a_negbit = self.register_a >> 7;
        let data_negbit = data >> 7;
        if data_negbit == 0 && register_a_negbit == 0 && sum >> 7 == 1
            || data_negbit == 1 && register_a_negbit == 1 && sum >> 7 == 0
        {
            self.flags.insert(CpuFlags::OVERFLOW);
        }

        if sum >> 7 ^ register_a_negbit == 0b1 {
            self.flags.insert(CpuFlags::CARRY);
        }
        self.set_register_a(sum);
    }

    fn and_with_register_a(&mut self, data: u8) {
        //todo remove this
        self.set_register_a(data & self.register_a);
    }

    fn xor_with_register_a(&mut self, data: u8) {
        //todo remove this
        self.set_register_a(data ^ self.register_a);
    }

    fn or_with_register_a(&mut self, data: u8) {
        //todo remove this
        self.set_register_a(data | self.register_a);
    }

    fn set_register_a(&mut self, data: u8) {
        self.register_a = data;
        self._udpate_cpu_flags(self.register_a);
    }

    fn set_register_x(&mut self, data: u8) {
        self.register_x = data;
        self._udpate_cpu_flags(self.register_x);
    }

    fn set_register_y(&mut self, data: u8) {
        self.register_y = data;
        self._udpate_cpu_flags(self.register_y);
    }

    fn _udpate_cpu_flags(&mut self, last_operation: u8) {
        if last_operation == 0 {
            self.flags.insert(CpuFlags::ZERO);
        } else {
            self.flags.remove(CpuFlags::ZERO);
        }
        self._update_negative_flag(last_operation);
    }

    fn _update_negative_flag(&mut self, last_operation: u8) {
        if last_operation >> 7 == 1 {
            self.flags.insert(CpuFlags::NEGATIV)
        } else {
            self.flags.remove(CpuFlags::NEGATIV)
        }
    }

    fn set_carry_flag(&mut self) {
        self.flags.insert(CpuFlags::CARRY)
    }

    fn clear_carry_flag(&mut self) {
        self.flags.remove(CpuFlags::CARRY)
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.memory
            .read((Memory::STACK as u16) + self.stack_pointer as u16)
    }

    fn stack_push(&mut self, data: u8) {
        self.memory
            .write((Memory::STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }

    pub fn interpret(&mut self, program: Vec<u8>) {
        let ref opscodes: HashMap<u8, &'static opscode::OpsCode> = *opscode::OPSCODES_MAP;

        let begin = self.program_counter as usize;
        let ops = opscodes.get(&program[begin]).unwrap();
        self.program_counter += 1;

        let program_counter_state = self.program_counter;

        match program[begin] {
            /* CLC */ 0x18 => {
                self.clear_carry_flag();
            }

            /* SEC */ 0x38 => {
                self.set_carry_flag();
            }

            /* PHA */ 0x48 => {
                self.stack_push(self.register_a);
            }

            /* PLA */
            0x68 => {
                let data = self.stack_pop();
                self.set_register_a(data);
            }

            /* ADC */
            0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                let data = ops.mode.read_u8(&program[..], self);
                self.add_to_register_a(data);
            }

            /* AND */
            0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                let data = ops.mode.read_u8(&program[..], self);
                self.and_with_register_a(data);
            }

            /* EOR */
            0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => {
                let data = ops.mode.read_u8(&program[..], self);
                self.xor_with_register_a(data);
            }

            0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => {
                let data = ops.mode.read_u8(&program[..], self);
                self.or_with_register_a(data);
            }

            /* LSR */
            0x4a | 0x46 | 0x56 | 0x4e | 0x5e => {
                let mut data = ops.mode.read_u8(&program[..], self);
                if data & 1 == 1 {
                    self.set_carry_flag();
                } else {
                    self.clear_carry_flag();
                }
                data = data >> 1;
                ops.mode.write_u8(&program[..], self, data);
                self._udpate_cpu_flags(data)
            }

            /* ASL */
            0x0a | 0x06 | 0x16 | 0x0e | 0x1e => {
                let mut data = ops.mode.read_u8(&program[..], self);
                if data >> 7 == 1 {
                    self.set_carry_flag();
                } else {
                    self.clear_carry_flag();
                }
                data = data << 1;
                ops.mode.write_u8(&program[..], self, data);
                self._udpate_cpu_flags(data)
            }

            /* ROL */
            0x2a | 0x26 | 0x36 | 0x2e | 0x3e => {
                let mut data = ops.mode.read_u8(&program[..], self);
                let old_carry = self.flags.contains(CpuFlags::CARRY);

                if data >> 7 == 1 {
                    self.set_carry_flag();
                } else {
                    self.clear_carry_flag();
                }
                data = data << 1;
                if old_carry {
                    data = data | 1;
                }
                ops.mode.write_u8(&program[..], self, data);
                self._update_negative_flag(data)
            }

            /* ROR */
            0x6a | 0x66 | 0x76 | 0x6e | 0x7e => {
                let mut data = ops.mode.read_u8(&program[..], self);
                let old_carry = self.flags.contains(CpuFlags::CARRY);

                if data & 1 == 1 {
                    self.set_carry_flag();
                } else {
                    self.clear_carry_flag();
                }
                data = data >> 1;
                if old_carry {
                    data = data | 0b10000000;
                }
                ops.mode.write_u8(&program[..], self, data);
                self._update_negative_flag(data)
            }

            /* INC */
            0xe6 | 0xf6 | 0xee | 0xfe => {
                let mut data = ops.mode.read_u8(&program[..], self);
                data = data.wrapping_add(1);
                ops.mode.write_u8(&program[..], self, data);
                self._udpate_cpu_flags(data);
            }
            /* INX */
            0xe8 => {
                self.register_x = self.register_x.wrapping_add(1);
                self._udpate_cpu_flags(self.register_x);
            }

            /* INY */
            0xc8 => {
                self.register_y = self.register_y.wrapping_add(1);
                self._udpate_cpu_flags(self.register_y);
            }

            /* DEC */
            0xc6 | 0xd6 | 0xce | 0xde => {
                //todo tests
                let mut data = ops.mode.read_u8(&program[..], self);
                data = data.wrapping_sub(1);
                ops.mode.write_u8(&program[..], self, data);
                self._udpate_cpu_flags(data);
            }

            /* DEX */
            0xca => {
                //todo tests
                self.register_x = self.register_x.wrapping_sub(1);
                self._udpate_cpu_flags(self.register_x);
            }

            /* DEY */
            0x88 => {
                //todo tests
                self.register_y = self.register_y.wrapping_sub(1);
                self._udpate_cpu_flags(self.register_y);
            }

            /* JMP Absolute */
            0x4c => {
                let mem_address = LittleEndian::read_u16(&program[self.program_counter as usize..]);
                self.program_counter = mem_address;
            }

            /* JMP Indirect */
            0x6c => {
                let mem_address = LittleEndian::read_u16(&program[self.program_counter as usize..]);
                let indirect_ref = self.memory.read_u16(mem_address);
                self.program_counter = indirect_ref;
                //todo: 6502 bug mode with with page boundary:
                //  if address $3000 contains $40, $30FF contains $80, and $3100 contains $50,
                // the result of JMP ($30FF) will be a transfer of control to $4080 rather than $5080 as you intended
                // i.e. the 6502 took the low byte of the address from $30FF and the high byte from $3000
            }

            /* STA */
            0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                ops.mode.write_u8(&program[..], self, self.register_a);
            }

            /* STX */
            0x86 | 0x96 | 0x8e => {
                //todo tests
                ops.mode.write_u8(&program[..], self, self.register_x);
            }

            /* STY */
            0x84 | 0x94 | 0x8c => {
                //todo tests
                ops.mode.write_u8(&program[..], self, self.register_y);
            }

            /* LDA */
            0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                //todo: tests
                let data = ops.mode.read_u8(&program[..], self);
                self.set_register_a(data);
            }

            /* LDX */
            0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => {
                let data = ops.mode.read_u8(&program[..], self);
                self.set_register_x(data);
            }

            /* LDY */
            0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => {
                let data = ops.mode.read_u8(&program[..], self);
                self.set_register_y(data);
            }

            /* NOP */
            0xea => {
                //do nothing
            }

            /* TAX */
            0xaa => {
                self.register_x = self.register_a;
                self._udpate_cpu_flags(self.register_x);
            }

            /* TAY */
            0xa8 => {
                self.register_y = self.register_a;
                self._udpate_cpu_flags(self.register_y);
            }

            /* TSX */
            0xba => {
                self.register_x = self.stack_pointer;
                self._udpate_cpu_flags(self.register_x);
            }

            /* TXA */
            0x8a => {
                self.register_a = self.register_x;
                self._udpate_cpu_flags(self.register_a);
            }

            /* TXS */
            0x9a => {
                self.stack_pointer = self.register_x;
            }

            /* TYA */
            0x98 => {
                self.register_a = self.register_y;
                self._udpate_cpu_flags(self.register_a);
            }

            _ => panic!("Unknown ops code"),
        }

        // todo: find more elegant way
        if program_counter_state == self.program_counter {
            self.program_counter += (ops.len - 1) as u16;
        }
        //todo: cycles

        if (self.program_counter as usize) < program.len() {
            self.interpret(program)
        }
    }

    pub fn new() -> Self {
        return CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: 0xFF,
            program_counter: 0,
            flags: CpuFlags::from_bits_truncate(0b00100000),
            memory: Memory::new(),
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_transform() {
        assert_eq!(CPU::transform("a9 8d"), [169, 141]);
    }

    #[test]
    fn test_0xa9_load_into_register_a() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("a9 8d"));
        assert_eq!(cpu.register_a, 0x8d);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_larger_program() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform(
            "a9 01 8d 00 02 a9 05 8d 01 02 a9 08 8d 02 02",
        ));
        assert_eq!(cpu.memory.read(0x0200), 01);
        assert_eq!(cpu.memory.read(0x0201), 05);
        assert_eq!(cpu.memory.read(0x0202), 08);
        assert_eq!(cpu.program_counter, 15);
    }

    #[test]
    fn test_0x48_pha() {
        let mut cpu = CPU::new();
        cpu.register_a = 100;
        cpu.interpret(CPU::transform("48"));
        assert_eq!(cpu.stack_pointer, 0xFE);
        assert_eq!(cpu.memory.read(Memory::STACK + 0xFF), 100);
        assert_eq!(cpu.program_counter, 1);
    }

    #[test]
    fn test_0x68_pla() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("a9 ff 48 a9 00 68"));
        assert_eq!(cpu.stack_pointer, 0xFF);
        assert_eq!(cpu.register_a, 0xff);
        assert_eq!(cpu.program_counter, 6);
    }

    #[test]
    fn test_0x48_pla_flags() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("a9 00 48 a9 01 68"));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_stack_overflowing() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("68"));
    }

    #[test]
    fn test_0x18_clc() {
        let mut cpu = CPU::new();
        cpu.flags.insert(CpuFlags::CARRY);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        cpu.interpret(CPU::transform("18"));
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        assert_eq!(cpu.program_counter, 1);
    }

    #[test]
    fn test_0x38_sec() {
        let mut cpu = CPU::new();
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        cpu.interpret(CPU::transform("38"));
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert_eq!(cpu.program_counter, 1);
    }

    #[test]
    fn test_0x85_sta() {
        let mut cpu = CPU::new();
        cpu.register_a = 101;
        cpu.interpret(CPU::transform("85 10"));
        assert_eq!(cpu.memory.read(0x10), 101);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_0x95_sta() {
        let mut cpu = CPU::new();
        cpu.register_a = 101;
        cpu.register_x = 0x50;
        cpu.interpret(CPU::transform("95 10"));
        assert_eq!(cpu.memory.read(0x60), 101);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_0x8d_sta() {
        let mut cpu = CPU::new();
        cpu.register_a = 100;
        cpu.interpret(CPU::transform("8d 00 02"));
        assert_eq!(cpu.memory.read(0x0200), 100);
        assert_eq!(cpu.program_counter, 3);
    }

    #[test]
    fn test_0x9d_sta_absolute_x() {
        let mut cpu = CPU::new();
        cpu.register_a = 101;
        cpu.register_x = 0x50;
        cpu.interpret(CPU::transform("9d 00 11"));
        assert_eq!(cpu.memory.read(0x1150), 101);
        assert_eq!(cpu.program_counter, 3);
    }

    #[test]
    fn test_0x99_sta_absolute_y() {
        let mut cpu = CPU::new();
        cpu.register_a = 101;
        cpu.register_y = 0x66;
        cpu.interpret(CPU::transform("99 00 11"));
        assert_eq!(cpu.memory.read(0x1166), 101);
        assert_eq!(cpu.program_counter, 3);
    }

    #[test]
    fn test_0x81_sta() {
        let mut cpu = CPU::new();
        cpu.register_x = 2;
        cpu.memory.write(0x2, 0x05);
        cpu.memory.write(0x3, 0x07);

        cpu.register_a = 0x66;

        cpu.interpret(CPU::transform("81 00"));
        assert_eq!(cpu.memory.read(0x0705), 0x66);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_091_sta() {
        let mut cpu = CPU::new();
        cpu.register_y = 0x10;
        cpu.memory.write(0x2, 0x05);
        cpu.memory.write(0x3, 0x07);

        cpu.register_a = 0x66;

        cpu.interpret(CPU::transform("91 02"));
        assert_eq!(cpu.memory.read(0x0705 + 0x10), 0x66);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_0x69_adc() {
        let mut cpu = CPU::new();
        cpu.register_a = 0x10;
        cpu.interpret(CPU::transform("69 02"));
        assert_eq!(cpu.register_a, 0x12);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn test_0x69_adc_carry_zero_flag() {
        let mut cpu = CPU::new();
        cpu.register_a = 0x81;
        cpu.interpret(CPU::transform("69 7f"));
        assert_eq!(cpu.register_a, 0x0);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert!(!cpu.flags.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0x69_adc_overflow_cary_flag() {
        let mut cpu = CPU::new();
        cpu.register_a = 0x8a;
        cpu.interpret(CPU::transform("69 8a"));
        assert_eq!(cpu.register_a, 0x14);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0x29_and_flags() {
        let mut cpu = CPU::new();
        cpu.register_a = 0b11010010;
        cpu.interpret(CPU::transform("29 90")); //0b10010000
        assert_eq!(cpu.register_a, 0b10010000);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x49_eor_flags() {
        let mut cpu = CPU::new();
        cpu.register_a = 0b11010010;
        cpu.interpret(CPU::transform("49 07")); //0b00000111
        assert_eq!(cpu.register_a, 0b11010101);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x09_ora_flags() {
        let mut cpu = CPU::new();
        cpu.register_a = 0b11010010;
        cpu.interpret(CPU::transform("09 07")); //0b00000111
        assert_eq!(cpu.register_a, 0b11010111);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x0a_asl_accumulator() {
        let mut cpu = CPU::new();
        cpu.register_a = 0b11010010;
        cpu.interpret(CPU::transform("0a"));
        assert_eq!(cpu.program_counter, 1);
        assert_eq!(cpu.register_a, 0b10100100);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x06_asl_memory() {
        let mut cpu = CPU::new();
        cpu.memory.write(0x10, 0b01000001);
        cpu.interpret(CPU::transform("06 10"));
        assert_eq!(cpu.memory.read(0x10), 0b10000010);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0x06_asl_memory_flags() {
        let mut cpu = CPU::new();
        cpu.memory.write(0x10, 0b10000000);
        cpu.interpret(CPU::transform("06 10"));
        assert_eq!(cpu.memory.read(0x10), 0b0);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert!(!cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0xf6_inc_memory_zero_page_x() {
        let mut cpu = CPU::new();
        cpu.register_x = 1;
        cpu.memory.write(0x10, 127);
        cpu.interpret(CPU::transform("f6 0f"));
        assert_eq!(cpu.memory.read(0x10), 128);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0x46_lsr_memory_flags() {
        let mut cpu = CPU::new();
        cpu.memory.write(0x10, 0b00000001);
        cpu.interpret(CPU::transform("46 10"));
        assert_eq!(cpu.memory.read(0x10), 0b0);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert!(!cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0x2e_rol_memory_flags() {
        let mut cpu = CPU::new();
        cpu.memory.write(0x1510, 0b10000001);
        cpu.interpret(CPU::transform("2e 10 15"));
        assert_eq!(cpu.memory.read(0x1510), 0b00000010);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2e_rol_memory_flags_carry() {
        let mut cpu = CPU::new();
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.memory.write(0x1510, 0b00000001);
        cpu.interpret(CPU::transform("2e 10 15"));
        assert_eq!(cpu.memory.read(0x1510), 0b00000011);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6e_ror_memory_flags_carry() {
        let mut cpu = CPU::new();
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.memory.write(0x1510, 0b01000010);
        cpu.interpret(CPU::transform("6e 10 15"));
        assert_eq!(cpu.memory.read(0x1510), 0b10100001);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6e_zero_flag() {
        let mut cpu = CPU::new();
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.memory.write(0x1510, 0b00000001);
        cpu.interpret(CPU::transform("6e 10 15"));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x6a_ror_accumulator_zero_falg() {
        let mut cpu = CPU::new();
        cpu.register_a = 1;
        cpu.interpret(CPU::transform("6a"));
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert_eq!(cpu.register_a, 0);
    }

    #[test]
    fn test_0xbe_ldx_absolute_y() {
        let mut cpu = CPU::new();
        cpu.memory.write(0x1166, 55);
        cpu.register_y = 0x66;
        cpu.interpret(CPU::transform("be 00 11"));
        assert_eq!(cpu.register_x, 55);
    }

    #[test]
    fn test_0xb4_ldy_zero_page_x() {
        let mut cpu = CPU::new();
        cpu.memory.write(0x66, 55);
        cpu.register_x = 0x06;
        cpu.interpret(CPU::transform("b4 60"));
        assert_eq!(cpu.register_y, 55);
    }

    #[test]
    fn test_0xc8_iny() {
        let mut cpu = CPU::new();
        cpu.register_y = 127;
        cpu.interpret(CPU::transform("c8"));
        assert_eq!(cpu.register_y, 128);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0xe8_inx() {
        let mut cpu = CPU::new();
        cpu.register_x = 0xff;
        cpu.interpret(CPU::transform("e8"));
        assert_eq!(cpu.register_x, 0);
        assert!(cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x6c_jmp_indirect() {
        let mut cpu = CPU::new();
        cpu.memory.write(0x0120, 0xfc);
        cpu.memory.write(0x0121, 0xba);
        cpu.interpret(CPU::transform("6c 20 01"));
        assert_eq!(cpu.program_counter, 0xbafc);
    }

    #[test]
    fn test_0x4c_jmp_absolute() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("4c 34 12"));
        assert_eq!(cpu.program_counter, 0x1234);
    }

    #[test]
    fn test_0xea_nop() {
        let mut cpu = CPU::new();
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.flags.insert(CpuFlags::NEGATIV);
        let flags = cpu.flags.clone();
        cpu.register_y = 1;
        cpu.register_x = 2;
        cpu.register_a = 3;

        cpu.interpret(CPU::transform("ea"));
        assert_eq!(cpu.program_counter, 1);
        assert_eq!(cpu.register_y, 1);
        assert_eq!(cpu.register_x, 2);
        assert_eq!(cpu.register_a, 3);
        assert_eq!(cpu.register_a, 3);
        assert_eq!(cpu.flags, flags);
    }

    #[test]
    fn test_0xaa_tax() {
        let mut cpu = CPU::new();
        cpu.register_a = 66;
        cpu.interpret(CPU::transform("aa"));
        assert_eq!(cpu.register_x, 66);
    }

    #[test]
    fn test_0xa8_tay() {
        let mut cpu = CPU::new();
        cpu.register_a = 66;
        cpu.interpret(CPU::transform("a8"));
        assert_eq!(cpu.register_y, 66);
    }

    #[test]
    fn test_0xba_tsx() {
        let mut cpu = CPU::new();
        cpu.interpret(CPU::transform("ba"));
        assert_eq!(cpu.register_x, 0xff);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0x8a_txa() {
        let mut cpu = CPU::new();
        cpu.register_x = 66;
        cpu.interpret(CPU::transform("8a"));
        assert_eq!(cpu.register_a, 66);
    }

    #[test]
    fn test_0x9a_txs() {
        let mut cpu = CPU::new();
        cpu.register_x = 0;
        cpu.interpret(CPU::transform("9a"));
        assert_eq!(cpu.stack_pointer, 0);
        assert!(!cpu.flags.contains(CpuFlags::ZERO)); // should not affect flags
    }

    #[test]
    fn test_0x98_tya() {
        let mut cpu = CPU::new();
        cpu.register_y = 66;
        cpu.interpret(CPU::transform("98"));
        assert_eq!(cpu.register_a, 66);
    }
}
