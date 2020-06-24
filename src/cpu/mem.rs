use crate::bus::bus::CpuBus;
use crate::cpu::cpu::CPU;

const ZERO_PAGE: u16 = 0x0;

pub trait Mem {
    fn write(&mut self, pos: u16, data: u8);
    fn write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write(pos, lo);
        self.write(pos + 1, hi);
    }

    fn read(&mut self, pos: u16) -> u8;

    fn read_u16(&mut self, pos: u16) -> u16 {
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
    Absolute_X_PageCross,
    Absolute_Y,
    Absolute_Y_PageCross,
    Indirect_X,
    Indirect_Y,
    Indirect_Y_PageCross,
    NoneAddressing,
}

impl AddressingMode {
    pub fn get_absolute_addr(&self, cpu: &mut CPU, base: u16) -> (bool, u16) {
        match self {
            AddressingMode::ZeroPage => (false, ZERO_PAGE + base),
            AddressingMode::ZeroPage_X => {
                let pos = (ZERO_PAGE + base) as u8;
                let addr = pos.wrapping_add(cpu.register_x) as u16;
                ((addr as u8) < pos, addr)
            }
            AddressingMode::ZeroPage_Y => {
                let pos = (ZERO_PAGE + base) as u8;
                let addr = pos.wrapping_add(cpu.register_y) as u16;
                ((addr as u8) < pos, addr)
            }
            AddressingMode::Absolute => (false, base),
            AddressingMode::Absolute_X | AddressingMode::Absolute_X_PageCross => {
                let addr = base.wrapping_add(cpu.register_x as u16);
                (page_cross(base, addr), addr)
            }
            AddressingMode::Absolute_Y | AddressingMode::Absolute_Y_PageCross => {
                let addr = base.wrapping_add(cpu.register_y as u16);
                (page_cross(base, addr), addr)
            }

            AddressingMode::Indirect_X => {
                let ptr: u8 = (base as u8).wrapping_add(cpu.register_x);
                let lo = cpu.mem_read(ptr as u16);
                let hi = cpu.mem_read(ptr.wrapping_add(1) as u16);
                (false, (hi as u16) << 8 | (lo as u16))
            }
            AddressingMode::Indirect_Y | AddressingMode::Indirect_Y_PageCross => {
                let lo = cpu.mem_read(base as u16);
                let hi = cpu.mem_read((base as u8).wrapping_add(1) as u16);
                // let deref = ((hi as u16) << 8 | (lo as u16)).wrapping_add(cpu.register_y as u16);

                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(cpu.register_y as u16);
                (page_cross(deref_base, deref), deref)
            }
            AddressingMode::Accumulator
            | AddressingMode::Immediate
            | AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", self);
            }
        }
    }

    pub fn read_u8<'a>(&self, cpu: &mut CPU) -> u8 {
        if let AddressingMode::Accumulator = self {
            return cpu.register_a;
        }

        if let AddressingMode::Immediate = self {
            return cpu.mem_read(cpu.program_counter);
        }

        let base = match self {
            AddressingMode::Absolute
            | AddressingMode::Absolute_X
            | AddressingMode::Absolute_Y
            | AddressingMode::Absolute_Y_PageCross
            | AddressingMode::Absolute_X_PageCross => cpu.mem_read_u16(cpu.program_counter),
            _ => cpu.mem_read(cpu.program_counter) as u16,
        };

        let (page_crossed, addr) = self.get_absolute_addr(cpu, base);
        if page_crossed && page_cross_mode(self) {
            cpu.bus.tick(1);
        }
        cpu.mem_read(addr)
    }

    pub fn write_u8(&self, cpu: &mut CPU, data: u8) {
        if let AddressingMode::Accumulator = self {
            cpu.set_register_a(data);
            return;
        }

        let argument = match self {
            AddressingMode::Absolute
            | AddressingMode::Absolute_X
            | AddressingMode::Absolute_Y
            | AddressingMode::Absolute_Y_PageCross
            | AddressingMode::Absolute_X_PageCross => cpu.mem_read_u16(cpu.program_counter),
            _ => cpu.mem_read(cpu.program_counter) as u16,
        };

        let (_page_cross, addr) = self.get_absolute_addr(cpu, argument);
        cpu.mem_write(addr, data);
    }
}

fn page_cross_mode(mode: &AddressingMode) -> bool {
    match mode {
        AddressingMode::Absolute_X_PageCross
        | AddressingMode::Absolute_Y_PageCross
        | AddressingMode::Indirect_Y_PageCross => true,
        _ => false,
    }
}

fn page_cross(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xFF00 != addr2 & 0xFF00
}
