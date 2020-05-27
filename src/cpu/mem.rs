use crate::cpu::cpu::CPU;
use byteorder::{ByteOrder, LittleEndian};
use std::cell::RefCell;
use std::rc::Rc;

const ZERO_PAGE: u16 = 0x0;


pub struct Memory {
    pub space: [u8; 0x10000],
    pub nmi_interrupt: Option<u8>,
}

pub trait Mem {
    fn write(&mut self, pos: u16, data: u8);
    fn write_u16(&mut self, pos: u16, data: u16);

    fn read(&self, pos: u16) -> u8;
    fn read_u16(&self, pos: u16) -> u16;

    fn poll_nmi_status(&mut self) -> Option<u8>;
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
     
    fn poll_nmi_status(&mut self) -> Option<u8> { 
        self.nmi_interrupt.take()
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

    fn poll_nmi_status(&mut self) -> std::option::Option<u8> { 
        self.mem.borrow_mut().poll_nmi_status() 
    }
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            space: [0; 0x10000],
            nmi_interrupt: None,
        }
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
