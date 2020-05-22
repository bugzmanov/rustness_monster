// https://skilldrick.github.io/easy6502/
use crate::cpu::opscode;
use byteorder::{ByteOrder, LittleEndian};
use hex;
use std::collections::HashMap;

use std::cell::RefCell;
use std::rc::Rc;

bitflags! {

/// # Status Register (P) http://wiki.nesdev.com/w/index.php/Status_flags
///
///  7 6 5 4 3 2 1 0
///  N V _ B D I Z C
///  | |   | | | | +--- Carry Flag
///  | |   | | | +----- Zero Flag
///  | |   | | +------- Interrupt Disable
///  | |   | +--------- Decimal Mode (not used on NES)
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

pub struct Memory {
    pub space: [u8; 0xffff],
}

const ZERO_PAGE: u16 = 0x0;
const STACK: u16 = 0x0100;
const STACK_SIZE: u8 = 0xff;

pub trait Mem {
    fn write(&mut self, pos: u16, data: u8);
    fn write_u16(&mut self, pos: u16, data: u16);

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

    fn write_u16(&mut self, pos: u16, data: u16) {
        LittleEndian::write_u16(&mut self.space[pos as usize..], data)
    }
}
pub struct DynamicMemWrapper {
    mem: Rc<RefCell<dyn Mem>>,
}

impl DynamicMemWrapper {
    pub fn new(data: Rc<RefCell<dyn Mem>>) -> Self {
        DynamicMemWrapper { mem: data }
    }
}

impl Mem for DynamicMemWrapper {
    fn write(&mut self, pos: u16, data: u8) {
        self.mem.borrow_mut().write(pos, data);
    }

    fn write_u16(&mut self, pos: u16, data: u16) {
        self.mem.borrow_mut().write_u16(pos, data);
    }
    fn read(&self, pos: u16) -> u8 {
        self.mem.borrow().read(pos)
    }
    fn read_u16(&self, pos: u16) -> u16 {
        self.mem.borrow().read_u16(pos)
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
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

impl AddressingMode {
    pub fn read_u8<'a>(&self, cpu: &CPU<'a>) -> u8 {
        if let AddressingMode::Accumulator = self {
            return cpu.register_a;
        }

        let pos: u8 = cpu.mem_read(cpu.program_counter);
        match self {
            AddressingMode::Immediate => pos,
            AddressingMode::ZeroPage => cpu.mem_read(ZERO_PAGE + pos as u16),
            AddressingMode::ZeroPage_X => {
                cpu.mem_read(ZERO_PAGE + pos as u16 + cpu.register_x as u16)
            }
            AddressingMode::ZeroPage_Y => {
                cpu.mem_read(ZERO_PAGE + pos as u16 + cpu.register_y as u16)
            }
            AddressingMode::Absolute => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter);
                cpu.mem_read(mem_address)
            }
            AddressingMode::Absolute_X => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter) + cpu.register_x as u16;
                cpu.mem_read(mem_address)
            }
            AddressingMode::Absolute_Y => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter) + cpu.register_y as u16;
                cpu.mem_read(mem_address)
            }

            AddressingMode::Indirect_X => {
                let ptr: u8 = pos + cpu.register_x; //todo overflow
                let deref = cpu.mem_read_u16(ptr as u16);
                cpu.mem_read(deref)
            }
            AddressingMode::Indirect_Y => {
                let deref = cpu.mem_read_u16(pos as u16) + cpu.register_y as u16;
                cpu.mem_read(deref)
            }
            AddressingMode::Accumulator => panic!("should not reach this code"),
            AddressingMode::NoneAddressing => {
                panic!("AddressingMode::NoneAddressing shouldn't be used to read data")
            }
        }
    }

    pub fn write_u8(&self, cpu: &mut CPU, data: u8) {
        if let AddressingMode::Accumulator = self {
            cpu.set_register_a(data);
            return;
        }

        let pos: u8 = cpu.mem_read(cpu.program_counter);

        match self {
            AddressingMode::Immediate => panic!("Immediate adressing mode is only for reading"),
            AddressingMode::ZeroPage => cpu.mem_write(pos as u16, data),
            AddressingMode::ZeroPage_X => cpu.mem_write((pos + cpu.register_x) as u16, data),
            AddressingMode::ZeroPage_Y => cpu.mem_write((pos + cpu.register_y) as u16, data),
            AddressingMode::Absolute => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter);
                cpu.mem_write(mem_address, data)
            }
            AddressingMode::Absolute_X => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter) + cpu.register_x as u16;
                cpu.mem_write(mem_address, data)
            }
            AddressingMode::Absolute_Y => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter) + cpu.register_y as u16;
                cpu.mem_write(mem_address, data)
            }
            AddressingMode::Indirect_X => {
                let ptr: u8 = pos + cpu.register_x; //todo overflow
                let deref = cpu.mem_read_u16(ptr as u16);
                cpu.mem_write(deref, data)
            }
            AddressingMode::Indirect_Y => {
                let deref = cpu.mem_read_u16(pos as u16) + cpu.register_y as u16;
                cpu.mem_write(deref, data)
            }
            AddressingMode::Accumulator => {
                panic!("shouldn't be here");
            }
            AddressingMode::NoneAddressing => {
                panic!("AddressingMode::NoneAddressing shouldn't be used to write data")
            }
        }
    }
}

pub struct CPU<'a> {
    register_a: u8,
    register_x: u8,
    register_y: u8,
    stack_pointer: u8,
    pub program_counter: u16,
    flags: CpuFlags,
    pub memory: &'a mut dyn Mem,
}

impl<'a> CPU<'a> {
    pub fn transform(s: &str) -> Vec<u8> {
        hex::decode(s.replace(' ', "")).expect("Decoding failed")
    }

    /// note: ignoring decimal mode
    /// http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html
    fn add_to_register_a(&mut self, data: u8) {
        let sum = self.register_a as u16
            + data as u16
            + (if self.flags.contains(CpuFlags::CARRY) {
                1
            } else {
                0
            }) as u16;

        let carry = sum > 0xff;

        if carry {
            self.flags.insert(CpuFlags::CARRY);
        } else {
            self.flags.remove(CpuFlags::CARRY);
        }

        let result = sum as u8;

        if (data ^ result) & (result ^ self.register_a) & 0x80 != 0 {
            self.flags.insert(CpuFlags::OVERFLOW);
        } else {
            self.flags.remove(CpuFlags::OVERFLOW)
        }

        self.set_register_a(result);
    }

    /// note: ignoring decimal mode
    fn sub_from_register_a(&mut self, data: u8) {
        self.add_to_register_a((-(data as i8) - 1) as u8);
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
        self.mem_read((STACK as u16) + self.stack_pointer as u16)
    }

    fn stack_push(&mut self, data: u8) {
        self.mem_write((STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }

    fn stack_push_u16(&mut self, data: u16) {
        self.mem_write_u16((STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(2);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        self.stack_pointer = self.stack_pointer.wrapping_add(2);
        self.mem_read_u16((STACK as u16) + self.stack_pointer as u16)
    }

    fn mem_read(&self, pos: u16) -> u8 {
        self.memory.read(pos)
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        self.memory.read_u16(pos)
    }

    fn mem_write(&mut self, pos: u16, data: u8) {
        self.memory.write(pos, data);
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        self.memory.write_u16(pos, data);
    }

    fn compare(&mut self, mode: &AddressingMode, compare_with: u8) {
        let data = mode.read_u8(self);
        if data <= compare_with {
            self.flags.insert(CpuFlags::CARRY);
        }

        self._udpate_cpu_flags(compare_with.wrapping_sub(data));
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            let jump: i8 = self.mem_read(self.program_counter) as i8;
            self.program_counter = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(jump as u16);
        }
    }

    pub fn interpret(&mut self, program: &[u8], mem_start: u16) {
        self.test_interpret_fn(program, mem_start, |_| {});
    }

    pub fn test_interpret_fn<F>(&mut self, program: &[u8], mem_start: u16, callback_opt: F)
    where
        F: FnMut(&mut CPU),
    {
        self.program_counter = mem_start;
        let mut pos = self.program_counter;
        for x in program {
            self.mem_write(pos, *x);
            pos += 1;
        }
        self.interpret_fn(mem_start as usize + program.len(), callback_opt);
    }

    pub fn interpret_fn<F>(&mut self, program_end: usize, mut callback_opt: F)
    //todo: program end is not needed
    where
        F: FnMut(&mut CPU),
    {
        let ref opscodes: HashMap<u8, &'static opscode::OpsCode> = *opscode::OPSCODES_MAP;
        while (self.program_counter as usize) < program_end {
            let code = self.mem_read(self.program_counter);
            let ops = opscodes.get(&code).unwrap();

            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            // debug
            // let tmp = match ops.len {
            //     2 => format!("{:x}", program[begin + 1]),
            //     3 => format!(
            //         "{:x}",
            //         LittleEndian::read_u16(&program[begin + 1 as usize..])
            //     ),
            //     _ => format!(""),
            // };
            // println!("{:x}: {} {}", self.program_counter, ops.mnemonic, tmp);

            match code {
                /* BRK */
                0x00 => {
                    self.flags.insert(CpuFlags::BREAK);
                    return;
                }

                /* CLD */ 0xd8 => self.flags.remove(CpuFlags::DECIMAL_MODE),

                /* CLI */ 0x58 => self.flags.remove(CpuFlags::INTERRUPT_DISABLE),

                /* CLV */ 0xb8 => self.flags.remove(CpuFlags::OVERFLOW),

                /* CLC */
                0x18 => {
                    self.clear_carry_flag();
                }

                /* SEC */ 0x38 => {
                    self.set_carry_flag();
                }

                /* SEI */
                0x78 => {
                    self.flags.insert(CpuFlags::INTERRUPT_DISABLE);
                }

                /* SED */
                0xf8 => {
                    self.flags.insert(CpuFlags::DECIMAL_MODE);
                }

                /* PHA */
                0x48 => {
                    self.stack_push(self.register_a);
                }

                /* PLA */
                0x68 => {
                    let data = self.stack_pop();
                    self.set_register_a(data);
                }

                /* PHP */
                0x08 => {
                    self.stack_push(self.flags.bits);
                }

                /* PLP */
                0x28 => {
                    self.flags.bits = self.stack_pop();
                }

                /* ADC */
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                    let data = ops.mode.read_u8(self);
                    self.add_to_register_a(data);
                }

                /* SBC */
                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                    let data = ops.mode.read_u8(self);
                    self.sub_from_register_a(data);
                }

                /* AND */
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                    let data = ops.mode.read_u8(self);
                    self.and_with_register_a(data);
                }

                /* EOR */
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => {
                    let data = ops.mode.read_u8(self);
                    self.xor_with_register_a(data);
                }

                /* ORA */
                0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => {
                    let data = ops.mode.read_u8(self);
                    self.or_with_register_a(data);
                }

                /* LSR */
                0x4a | 0x46 | 0x56 | 0x4e | 0x5e => {
                    let mut data = ops.mode.read_u8(self);
                    if data & 1 == 1 {
                        self.set_carry_flag();
                    } else {
                        self.clear_carry_flag();
                    }
                    data = data >> 1;
                    ops.mode.write_u8(self, data);
                    self._udpate_cpu_flags(data)
                }

                /* ASL */
                0x0a | 0x06 | 0x16 | 0x0e | 0x1e => {
                    let mut data = ops.mode.read_u8(self);
                    if data >> 7 == 1 {
                        self.set_carry_flag();
                    } else {
                        self.clear_carry_flag();
                    }
                    data = data << 1;
                    ops.mode.write_u8(self, data);
                    self._udpate_cpu_flags(data)
                }

                /* ROL */
                0x2a | 0x26 | 0x36 | 0x2e | 0x3e => {
                    let mut data = ops.mode.read_u8(self);
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
                    ops.mode.write_u8(self, data);
                    self._update_negative_flag(data)
                }

                /* ROR */
                0x6a | 0x66 | 0x76 | 0x6e | 0x7e => {
                    let mut data = ops.mode.read_u8(self);
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
                    ops.mode.write_u8(self, data);
                    self._update_negative_flag(data)
                }

                /* INC */
                0xe6 | 0xf6 | 0xee | 0xfe => {
                    let mut data = ops.mode.read_u8(self);
                    data = data.wrapping_add(1);
                    ops.mode.write_u8(self, data);
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
                    let mut data = ops.mode.read_u8(self);
                    data = data.wrapping_sub(1);
                    ops.mode.write_u8(self, data);
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

                /* CMP */
                0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => {
                    self.compare(&ops.mode, self.register_a);
                }

                /* CPY */ //todo tests
                0xc0 | 0xc4 | 0xcc => {
                    self.compare(&ops.mode, self.register_y);
                }

                /* CPX */ //todo tests
                0xe0 | 0xe4 | 0xec => self.compare(&ops.mode, self.register_x),

                /* JMP Absolute */
                0x4c => {
                    let mem_address = self.mem_read_u16(self.program_counter);
                    self.program_counter = mem_address;
                }

                /* JMP Indirect */
                0x6c => {
                    let mem_address = self.mem_read_u16(self.program_counter);
                    let indirect_ref = self.mem_read_u16(mem_address);
                    self.program_counter = indirect_ref;
                    //todo: 6502 bug mode with with page boundary:
                    //  if address $3000 contains $40, $30FF contains $80, and $3100 contains $50,
                    // the result of JMP ($30FF) will be a transfer of control to $4080 rather than $5080 as you intended
                    // i.e. the 6502 took the low byte of the address from $30FF and the high byte from $3000
                }

                /* JSR */
                0x20 => {
                    self.stack_push_u16(self.program_counter + 2 - 1);
                    let target_address = self.mem_read_u16(self.program_counter);
                    self.program_counter = target_address
                }

                /* RTS */
                0x60 => {
                    self.program_counter = self.stack_pop_u16() + 1;
                }

                /* RTI */
                0x40 => {
                    self.flags.bits = self.stack_pop();
                    self.program_counter = self.stack_pop_u16();
                }

                /* BNE */
                0xd0 => {
                    self.branch(!self.flags.contains(CpuFlags::ZERO));
                }

                /* BVS */
                0x70 => {
                    self.branch(self.flags.contains(CpuFlags::OVERFLOW));
                }

                /* BVC */
                0x50 => {
                    self.branch(!self.flags.contains(CpuFlags::OVERFLOW));
                }

                /* BPL */
                0x10 => {
                    self.branch(!self.flags.contains(CpuFlags::NEGATIV));
                }

                /* BMI */
                0x30 => {
                    self.branch(self.flags.contains(CpuFlags::NEGATIV));
                }

                /* BEQ */
                0xf0 => {
                    self.branch(self.flags.contains(CpuFlags::ZERO));
                }

                /* BCS */
                0xb0 => {
                    self.branch(self.flags.contains(CpuFlags::CARRY));
                }

                /* BCC */
                0x90 => {
                    self.branch(!self.flags.contains(CpuFlags::CARRY));
                }

                /* BIT */
                0x24 | 0x2c => {
                    let data = ops.mode.read_u8(self);
                    let and = self.register_a & data;
                    if and == 0 {
                        self.flags.insert(CpuFlags::ZERO);
                    } else {
                        self.flags.remove(CpuFlags::ZERO);
                    }

                    self.flags.set(CpuFlags::NEGATIV, data & 0b10000000 > 0);
                    self.flags.set(CpuFlags::OVERFLOW, data & 0b01000000 > 0);
                }

                /* STA */
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                    ops.mode.write_u8(self, self.register_a);
                }

                /* STX */
                0x86 | 0x96 | 0x8e => {
                    //todo tests
                    ops.mode.write_u8(self, self.register_x);
                }

                /* STY */
                0x84 | 0x94 | 0x8c => {
                    //todo tests
                    ops.mode.write_u8(self, self.register_y);
                }

                /* LDA */
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                    //todo: tests
                    let data = ops.mode.read_u8(self);
                    self.set_register_a(data);
                }

                /* LDX */
                0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => {
                    let data = ops.mode.read_u8(self);
                    self.set_register_x(data);
                }

                /* LDY */
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => {
                    let data = ops.mode.read_u8(self);
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

            // if there were no jumps, advance program counter
            // todo: find more elegant way
            if program_counter_state == self.program_counter {
                self.program_counter += (ops.len - 1) as u16;
            }
            //todo: cycles

            callback_opt(self);
        }
    }

    pub fn new<'b>(_mem: &'b mut dyn Mem) -> CPU<'b> {
        return CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: STACK_SIZE,
            program_counter: 0,
            flags: CpuFlags::from_bits_truncate(0b00100000),
            memory: _mem,
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
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.interpret(&CPU::transform("a9 8d"), 100);
        assert_eq!(cpu.register_a, 0x8d);
        assert_eq!(cpu.program_counter, 102);
    }

    #[test]
    fn test_larger_program() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.interpret(
            &CPU::transform("a9 01 8d 00 02 a9 05 8d 01 02 a9 08 8d 02 02"),
            100,
        );
        assert_eq!(cpu.mem_read(0x0200), 01);
        assert_eq!(cpu.mem_read(0x0201), 05);
        assert_eq!(cpu.mem_read(0x0202), 08);
        assert_eq!(cpu.program_counter, 115);
    }

    #[test]
    fn test_0x48_pha() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 100;
        cpu.interpret(&CPU::transform("48"), 100);
        assert_eq!(cpu.stack_pointer, 0xFE);
        assert_eq!(cpu.mem_read(STACK + 0xFF), 100);
        assert_eq!(cpu.program_counter, 101);
    }

    #[test]
    fn test_0x68_pla() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.interpret(&CPU::transform("a9 ff 48 a9 00 68"), 100);
        assert_eq!(cpu.stack_pointer, 0xFF);
        assert_eq!(cpu.register_a, 0xff);
        assert_eq!(cpu.program_counter, 106);
    }

    #[test]
    fn test_0x48_pla_flags() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.interpret(&CPU::transform("a9 00 48 a9 01 68"), 100);
        assert!(cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_stack_overflowing() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.interpret(&CPU::transform("68"), 100);
    }

    #[test]
    fn test_0x18_clc() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.flags.insert(CpuFlags::CARRY);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        cpu.interpret(&CPU::transform("18"), 100);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        assert_eq!(cpu.program_counter, 101);
    }

    #[test]
    fn test_0x38_sec() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        cpu.interpret(&CPU::transform("38"), 100);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert_eq!(cpu.program_counter, 101);
    }

    #[test]
    fn test_0x85_sta() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 101;
        cpu.interpret(&CPU::transform("85 10"), 100);
        assert_eq!(cpu.mem_read(0x10), 101);
        assert_eq!(cpu.program_counter, 102);
    }

    #[test]
    fn test_0x95_sta() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 101;
        cpu.register_x = 0x50;
        cpu.interpret(&CPU::transform("95 10"), 100);
        assert_eq!(cpu.mem_read(0x60), 101);
        assert_eq!(cpu.program_counter, 102);
    }

    #[test]
    fn test_0x8d_sta() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 100;
        cpu.interpret(&CPU::transform("8d 00 02"), 100);
        assert_eq!(cpu.mem_read(0x0200), 100);
        assert_eq!(cpu.program_counter, 103);
    }

    #[test]
    fn test_0x9d_sta_absolute_x() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 101;
        cpu.register_x = 0x50;
        cpu.interpret(&CPU::transform("9d 00 11"), 100);
        assert_eq!(cpu.mem_read(0x1150), 101);
        assert_eq!(cpu.program_counter, 103);
    }

    #[test]
    fn test_0x99_sta_absolute_y() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 101;
        cpu.register_y = 0x66;
        cpu.interpret(&CPU::transform("99 00 11"), 100);
        assert_eq!(cpu.mem_read(0x1166), 101);
        assert_eq!(cpu.program_counter, 103);
    }

    #[test]
    fn test_0x81_sta() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_x = 2;
        cpu.mem_write(0x2, 0x05);
        cpu.mem_write(0x3, 0x07);

        cpu.register_a = 0x66;

        cpu.interpret(&CPU::transform("81 00"), 100);
        assert_eq!(cpu.mem_read(0x0705), 0x66);
        assert_eq!(cpu.program_counter, 102);
    }

    #[test]
    fn test_091_sta() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_y = 0x10;
        cpu.mem_write(0x2, 0x05);
        cpu.mem_write(0x3, 0x07);

        cpu.register_a = 0x66;

        cpu.interpret(&CPU::transform("91 02"), 100);
        assert_eq!(cpu.mem_read(0x0705 + 0x10), 0x66);
        assert_eq!(cpu.program_counter, 102);
    }

    #[test]
    fn test_0x69_adc() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0x10;
        cpu.interpret(&CPU::transform("69 02"), 100);
        assert_eq!(cpu.register_a, 0x12);
        assert_eq!(cpu.program_counter, 102);
    }

    #[test]
    fn test_0x69_adc_carry_zero_flag() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0x81;
        cpu.interpret(&CPU::transform("69 7f"), 100);
        assert_eq!(cpu.register_a, 0x0);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert!(!cpu.flags.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0x69_adc_overflow_cary_flag() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0x8a;
        cpu.interpret(&CPU::transform("69 8a"), 100);
        assert_eq!(cpu.register_a, 0x14);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0xe9_sbc() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0x10;
        cpu.interpret(&CPU::transform("e9 02"), 100);
        assert_eq!(cpu.register_a, 0x0d);
        assert_eq!(cpu.program_counter, 102);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(!cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0xe9_sbc_negative() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0x02;
        cpu.interpret(&CPU::transform("e9 03"), 100);
        assert_eq!(cpu.register_a, 0xfe);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0xe9_sbc_overflow() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0x50;
        cpu.interpret(&CPU::transform("e9 b0"), 100);
        assert_eq!(cpu.register_a, 0x9f);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(cpu.flags.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_0x29_and_flags() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0b11010010;
        cpu.interpret(&CPU::transform("29 90"), 100); //0b10010000
        assert_eq!(cpu.register_a, 0b10010000);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x49_eor_flags() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0b11010010;
        cpu.interpret(&CPU::transform("49 07"), 100); //0b00000111
        assert_eq!(cpu.register_a, 0b11010101);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x09_ora_flags() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0b11010010;
        cpu.interpret(&CPU::transform("09 07"), 100); //0b00000111
        assert_eq!(cpu.register_a, 0b11010111);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x0a_asl_accumulator() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0b11010010;
        cpu.interpret(&CPU::transform("0a"), 100);
        assert_eq!(cpu.program_counter, 101);
        assert_eq!(cpu.register_a, 0b10100100);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x06_asl_memory() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.mem_write(0x10, 0b01000001);
        cpu.interpret(&CPU::transform("06 10"), 100);
        assert_eq!(cpu.mem_read(0x10), 0b10000010);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0x06_asl_memory_flags() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.mem_write(0x10, 0b10000000);
        cpu.interpret(&CPU::transform("06 10"), 100);
        assert_eq!(cpu.mem_read(0x10), 0b0);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert!(!cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0xf6_inc_memory_zero_page_x() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_x = 1;
        cpu.mem_write(0x10, 127);
        cpu.interpret(&CPU::transform("f6 0f"), 100);
        assert_eq!(cpu.mem_read(0x10), 128);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0x46_lsr_memory_flags() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.mem_write(0x10, 0b00000001);
        cpu.interpret(&CPU::transform("46 10"), 100);
        assert_eq!(cpu.mem_read(0x10), 0b0);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert!(!cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0x2e_rol_memory_flags() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.mem_write(0x1510, 0b10000001);
        cpu.interpret(&CPU::transform("2e 10 15"), 100);
        assert_eq!(cpu.mem_read(0x1510), 0b00000010);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x2e_rol_memory_flags_carry() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.mem_write(0x1510, 0b00000001);
        cpu.interpret(&CPU::transform("2e 10 15"), 100);
        assert_eq!(cpu.mem_read(0x1510), 0b00000011);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6e_ror_memory_flags_carry() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.mem_write(0x1510, 0b01000010);
        cpu.interpret(&CPU::transform("6e 10 15"), 100);
        assert_eq!(cpu.mem_read(0x1510), 0b10100001);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_0x6e_zero_flag() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.mem_write(0x1510, 0b00000001);
        cpu.interpret(&CPU::transform("6e 10 15"), 100);
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x6a_ror_accumulator_zero_falg() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 1;
        cpu.interpret(&CPU::transform("6a"), 100);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert_eq!(cpu.register_a, 0);
    }

    #[test]
    fn test_0xbe_ldx_absolute_y() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.mem_write(0x1166, 55);
        cpu.register_y = 0x66;
        cpu.interpret(&CPU::transform("be 00 11"), 100);
        assert_eq!(cpu.register_x, 55);
    }

    #[test]
    fn test_0xb4_ldy_zero_page_x() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.mem_write(0x66, 55);
        cpu.register_x = 0x06;
        cpu.interpret(&CPU::transform("b4 60"), 100);
        assert_eq!(cpu.register_y, 55);
    }

    #[test]
    fn test_0xc8_iny() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_y = 127;
        cpu.interpret(&CPU::transform("c8"), 100);
        assert_eq!(cpu.register_y, 128);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0xe8_inx() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_x = 0xff;
        cpu.interpret(&CPU::transform("e8"), 100);
        assert_eq!(cpu.register_x, 0);
        assert!(cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn test_0x6c_jmp_indirect() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.mem_write(0x0120, 0xfc);
        cpu.mem_write(0x0121, 0xba);
        cpu.interpret(&CPU::transform("6c 20 01"), 100);
        assert_eq!(cpu.program_counter, 0xbafc);
    }

    #[test]
    fn test_0x4c_jmp_absolute() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.interpret(&CPU::transform("4c 34 12"), 100);
        assert_eq!(cpu.program_counter, 0x1234);
    }

    #[test]
    fn test_0xea_nop() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.flags.insert(CpuFlags::NEGATIV);
        let flags = cpu.flags.clone();
        cpu.register_y = 1;
        cpu.register_x = 2;
        cpu.register_a = 3;

        cpu.interpret(&CPU::transform("ea"), 100);
        assert_eq!(cpu.program_counter, 101);
        assert_eq!(cpu.register_y, 1);
        assert_eq!(cpu.register_x, 2);
        assert_eq!(cpu.register_a, 3);
        assert_eq!(cpu.register_a, 3);
        assert_eq!(cpu.flags, flags);
    }

    #[test]
    fn test_0xaa_tax() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 66;
        cpu.interpret(&CPU::transform("aa"), 100);
        assert_eq!(cpu.register_x, 66);
    }

    #[test]
    fn test_0xa8_tay() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 66;
        cpu.interpret(&CPU::transform("a8"), 100);
        assert_eq!(cpu.register_y, 66);
    }

    #[test]
    fn test_0xba_tsx() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.interpret(&CPU::transform("ba"), 100);
        assert_eq!(cpu.register_x, 0xff);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0x8a_txa() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_x = 66;
        cpu.interpret(&CPU::transform("8a"), 100);
        assert_eq!(cpu.register_a, 66);
    }

    #[test]
    fn test_0x9a_txs() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_x = 0;
        cpu.interpret(&CPU::transform("9a"), 100);
        assert_eq!(cpu.stack_pointer, 0);
        assert!(!cpu.flags.contains(CpuFlags::ZERO)); // should not affect flags
    }

    #[test]
    fn test_0x98_tya() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_y = 66;
        cpu.interpret(&CPU::transform("98"), 100);
        assert_eq!(cpu.register_a, 66);
    }

    #[test]
    fn test_0x20_jsr() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        let pc = 100; //cpu.program_counter;
        cpu.interpret(&CPU::transform("20 04 06"), 100);
        assert_eq!(cpu.program_counter, 0x604);
        assert_eq!(cpu.stack_pointer, 0xff - 0x2);
        let return_pos = cpu.stack_pop_u16();
        assert_eq!(pc + 3 - 1, return_pos);
    }

    #[test]
    fn test_0x60_rts() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        /*
            JSR init
            BRK

            init:
            LDX #$05
            RTS
        */
        cpu.interpret(&CPU::transform("20 68 00 00 a2 05 60"), 100);
        assert_eq!(cpu.program_counter, 104);
        assert_eq!(cpu.stack_pointer, 0xff);
        assert_eq!(cpu.register_x, 0x5);
    }

    #[test]
    fn test_0x40_rti() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.flags.bits = 0b11000001;
        cpu.program_counter = 0x100;
        cpu.stack_push_u16(cpu.program_counter);
        cpu.stack_push(cpu.flags.bits);

        cpu.flags.bits = 0;
        cpu.program_counter = 0;
        cpu.interpret(&CPU::transform("40"), 100);

        assert_eq!(cpu.flags.bits, 0b11000001);
        assert_eq!(cpu.program_counter, 0x100);
    }

    #[test]
    fn test_0xc9_cmp_immidiate() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0x6;
        cpu.interpret(&CPU::transform("c9 05"), 100);
        assert!(cpu.flags.contains(CpuFlags::CARRY));

        cpu.program_counter = 0;
        cpu.flags.bits = 0;
        cpu.interpret(&CPU::transform("c9 06"), 100);
        assert!(cpu.flags.contains(CpuFlags::CARRY));
        assert!(cpu.flags.contains(CpuFlags::ZERO));

        cpu.program_counter = 0;
        cpu.flags.bits = 0;
        cpu.interpret(&CPU::transform("c9 07"), 100);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));

        cpu.program_counter = 0;
        cpu.flags.bits = 0;
        cpu.interpret(&CPU::transform("c9 90"), 100);
        assert!(!cpu.flags.contains(CpuFlags::CARRY));
        assert!(!cpu.flags.contains(CpuFlags::ZERO));
        assert!(!cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn test_0xd0_bne() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        // jump
        cpu.flags.remove(CpuFlags::ZERO);
        cpu.interpret(&CPU::transform("d0 04"), 100);
        assert_eq!(cpu.program_counter, 100 + 0x6);

        // no jump
        cpu.flags.insert(CpuFlags::ZERO);
        cpu.interpret(&CPU::transform("d0 04"), 100);
        assert_eq!(cpu.program_counter, 100 + 0x02);
    }

    #[test]
    fn test_0xd0_bne_snippet() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        /*
            LDX #$08
        decrement:
            DEX
            INY
            CPX #$03
            BNE decrement
            BRK
        */
        cpu.interpret(&CPU::transform("a2 08 ca c8 e0 03 d0 fa 00"), 100);
        assert_eq!(cpu.register_y, 5);
    }

    #[test]
    fn test_0x24_bit() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0b00000010;
        cpu.mem_write(0x10, 0b10111101);
        cpu.interpret(&CPU::transform("24 10"), 100);

        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_ololo() {
        let mut mem = Memory::new();
        let mut cpu = CPU::new(&mut mem);
        cpu.register_a = 0b00000010;
        cpu.mem_write(0x10, 0b10111101);
        cpu.interpret(&CPU::transform("24 10"), 100);

        assert!(cpu.flags.contains(CpuFlags::ZERO));
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert!(!cpu.flags.contains(CpuFlags::OVERFLOW));
    }
}
