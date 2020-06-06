use crate::cpu::cpu::CPU;
use crate::bus::bus::CpuBus;

const ZERO_PAGE: u16 = 0x0;

pub trait Mem {
    fn write(&mut self, pos: u16, data: u8);
    fn write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write(pos, lo);
        self.write(pos + 1, hi);
    }

    fn read(&self, pos: u16) -> u8;

    fn read_u16(&self, pos: u16) -> u16 {
        let lo = self.read(pos) as u16;
        let hi = self.read(pos + 1) as u16;
        (hi << 8) | (lo as u16)
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

    pub fn read_u8_from_pos(&self, cpu: &CPU, pos: u16) -> (u16, u8) {
        if let AddressingMode::Accumulator = self {
            return (cpu.register_a as u16, cpu.register_a);
        }

        // let pos: u8 = cpu.mem_read(cpu.program_counter);
        match self {
            AddressingMode::Immediate => (pos, pos as u8),
            AddressingMode::ZeroPage => {
                let addr = ZERO_PAGE + pos;
                (addr, cpu.mem_read(addr))  
            }
            AddressingMode::ZeroPage_X => {
                let pos = (ZERO_PAGE + pos) as u8;
                let addr = pos.wrapping_add(cpu.register_x) as u16;
                (addr, cpu.mem_read(addr as u16))
            }
            AddressingMode::ZeroPage_Y => {
                let pos = (ZERO_PAGE + pos) as u8;
                let addr = pos.wrapping_add(cpu.register_y) as u16;
                (addr, cpu.mem_read(addr))
            }
            AddressingMode::Absolute => {
                let addr = pos;
                (addr, cpu.mem_read(addr))
            }
            AddressingMode::Absolute_X => {
                let addr = pos.wrapping_add(cpu.register_x as u16);
                (addr, cpu.mem_read(addr))
            }
            AddressingMode::Absolute_Y => {
                let addr = pos.wrapping_add(cpu.register_y as u16);
                (addr, cpu.mem_read(addr))
            }

            AddressingMode::Indirect_X => {
                let ptr: u8 = (pos as u8).wrapping_add(cpu.register_x); //todo overflow
                let lo = cpu.mem_read(ptr as u16);
                let hi = cpu.mem_read(ptr.wrapping_add(1) as u16);
                // let deref = cpu.mem_read_u16(ptr as u16);
                let deref = (hi as u16) << 8 | (lo as u16);
                (deref, cpu.mem_read(deref))
            }
            AddressingMode::Indirect_Y => {
                let lo = cpu.mem_read(pos as u16);
                let hi = cpu.mem_read((pos as u8).wrapping_add(1) as u16);
                let deref = ((hi as u16) << 8 | (lo as u16)).wrapping_add(cpu.register_y as u16);

                (deref, cpu.mem_read(deref))
            }
            AddressingMode::Accumulator => panic!("should not reach this code"),
            AddressingMode::NoneAddressing => {
                panic!("AddressingMode::NoneAddressing shouldn't be used to read data")
            }
        }

    }

    pub fn read_u8<'a>(&self, cpu: &CPU) -> u8 {
        let pos = match self {
            AddressingMode::Absolute | AddressingMode::Absolute_X | AddressingMode::Absolute_Y =>
                cpu.mem_read_u16(cpu.program_counter),
            _ => cpu.mem_read(cpu.program_counter) as u16
        };
        self.read_u8_from_pos(cpu, pos ).1
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
            AddressingMode::ZeroPage_X => cpu.mem_write((pos.wrapping_add(cpu.register_x)) as u16, data),
            AddressingMode::ZeroPage_Y => cpu.mem_write((pos.wrapping_add(cpu.register_y)) as u16, data),
            AddressingMode::Absolute => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter);
                cpu.mem_write(mem_address, data)
            }
            AddressingMode::Absolute_X => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter).wrapping_add(cpu.register_x as u16);
                cpu.mem_write(mem_address, data)
            }
            AddressingMode::Absolute_Y => {
                let mem_address = cpu.mem_read_u16(cpu.program_counter).wrapping_add(cpu.register_y as u16);
                cpu.mem_write(mem_address, data)
            }
            AddressingMode::Indirect_X => {
                let ptr: u8 = (pos as u8).wrapping_add(cpu.register_x); 
                let lo = cpu.mem_read(ptr as u16);
                let hi = cpu.mem_read(ptr.wrapping_add(1) as u16);
                let deref = (hi as u16) << 8 | (lo as u16);
                cpu.mem_write(deref, data)
            }
            AddressingMode::Indirect_Y => {
                let lo = cpu.mem_read(pos as u16);
                let hi = cpu.mem_read((pos as u8).wrapping_add(1) as u16);
                let deref = ((hi as u16) << 8 | (lo as u16)).wrapping_add(cpu.register_y as u16);

                cpu.mem_write(deref as u16, data)
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
