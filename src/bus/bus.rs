use crate::cpu::mem::Mem;
use crate::ppu::ppu::NesPPU;
use crate::ppu::ppu::PPU;
use crate::rom::ines::Rom;
use std::cell::RefCell;
use std::rc::Rc;

// # Memory Map http://nesdev.com/NESDoc.pdf
//
//  _______________ $10000  _______________
// | PRG-ROM       |       |               |
// | Upper Bank    |       |               |
// |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
// | PRG-ROM       |       |               |
// | Lower Bank    |       |               |
// |_______________| $8000 |_______________|
// | SRAM          |       | SRAM          |
// |_______________| $6000 |_______________|
// | Expansion ROM |       | Expansion ROM |
// |_______________| $4020 |_______________|
// | I/O Registers |       |               |
// |_ _ _ _ _ _ _ _| $4000 |               |
// | Mirrors       |       | I/O Registers |
// | $2000-$2007   |       |               |
// |_ _ _ _ _ _ _ _| $2008 |               |
// | I/O Registers |       |               |
// |_______________| $2000 |_______________|
// | Mirrors       |       |               |
// | $0000-$07FF   |       |               |
// |_ _ _ _ _ _ _ _| $0800 |               |
// | RAM           |       | RAM           |
// |_ _ _ _ _ _ _ _| $0200 |               |
// | Stack         |       |               |
// |_ _ _ _ _ _ _ _| $0100 |               |
// | Zero Page     |       |               |
// |_______________| $0000 |_______________|
//

pub struct Bus<T: PPU> {
    pub ram: [u8; 0x800],
    pub rom: Rom,
    pub nmi_interrupt: Option<u8>,
    pub cycles: usize,
    ppu: RefCell<T>,
}

const ZERO_PAGE: u16 = 0x0;
const STACK: u16 = 0x0100;
const RAM: u16 = 0x0200;
const RAM_MIRRORS: u16 = 0x0800;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const IO_REGISTERS: u16 = 0x2000;
const IO_MIRRORS: u16 = 0x2008;
const IO_MIRRORS_END: u16 = 0x3FFF;
const PRG_ROM: u16 = 0x8000;
const PRG_ROM_END: u16 = 0xFFFF;

fn map_mirrors(pos: u16) -> u16 {
    match pos {
        RAM_MIRRORS..=RAM_MIRRORS_END => pos & 0b11111111111,
        IO_MIRRORS..=IO_MIRRORS_END => pos & 0b10000000000111,
        _ => pos,
    }
}

#[allow(dead_code)]
impl<T: PPU> Bus<T> {
    pub fn new(rom: Rom) -> Bus<NesPPU> {
        let chr_rom_copy = rom.chr_rom.clone(); // todo: this will bite me with mappers
        let mirroring = rom.rom_flags.mirroring();
        Bus {
            ram: [0; 2048],
            rom: rom,
            nmi_interrupt: None,
            cycles: 0,
            ppu: RefCell::from(NesPPU::new(chr_rom_copy, mirroring)),
        }
    }

    pub fn write(&mut self, pos: u16, data: u8) {
        match pos {
            0x00..=RAM_MIRRORS_END => {
                let pos = map_mirrors(pos);
                self.ram[pos as usize] = data;
            }
            0x2000 => {
                self.ppu.borrow_mut().write_to_ctrl(data);
            }
            0x2001 => {
                self.ppu.borrow_mut().write_to_mask(data);
            }

            0x2002 => panic!("attempt to write to PPU status register"),

            0x2003 => {
                self.ppu.borrow_mut().write_to_oam_addr(data);
            }
            0x2004 => {
                self.ppu.borrow_mut().write_to_oam_data(data);
            }
            0x2005 => {
                self.ppu.borrow_mut().write_to_scroll(data);
            }

            0x2006 => {
                self.ppu.borrow_mut().write_to_ppu_addr(data);
            }
            0x2007 => {
                self.ppu.borrow_mut().write_to_data(data);
            }
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (data as u16) << 8;
                for i in 0..255u16 {
                    buffer[i as usize] = self.read(hi + i);
                }

                self.ppu.borrow_mut().write_oam_dma(&buffer);
                let add_cycles: u16 = if self.cycles % 2 == 1 { 514 } else { 513 };
                self.tick(add_cycles); //todo this will cause weird effects as PPU will have 513/514 * 3 ticks
            }

            IO_MIRRORS..=IO_MIRRORS_END => {
                //mirror IO registers
                self.write(pos & 0b10000000000111, data)
            }

            0x4000..=0x4015 => {
                //todo: implement
                //ignore APU for now
            }

            0x4016 => {
                //todo: implement
                //ignore joypad 1 for now
            }

            0x4017 => {
                //todo: implement
                //ignore joypad 2 for now
            }

            //todo 0x4000 - 0x8000
            PRG_ROM..=PRG_ROM_END => {
                panic!("attempt to write to a ROM section: {:x}", pos); //sram?
            }
            _ => {
                unimplemented!("attempting to write to {:x}", pos);
            }
        }
    }

    pub fn read(&self, pos: u16) -> u8 {
        match pos {
            0x0..=RAM_MIRRORS_END => {
                let pos = map_mirrors(pos);
                self.ram[pos as usize]
            }
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => {
                panic!("Attempt to read from write-only PPU address {:x}", pos);
            }
            0x2002 => self.ppu.borrow_mut().read_status(),
            0x2004 => self.ppu.borrow().read_oam_data(),
            0x2007 => self.ppu.borrow_mut().read_data(),

            IO_MIRRORS..=IO_MIRRORS_END => {
                //mirror IO registers
                self.read(pos & 0b10000000000111)
            }
            0x4000..=0x4013 => panic!("Attempt to read from write-only APU address {:x}", pos),

            0x4015 => {
                //todo: implement APU register
                0
            }

            0x4016 => {
                //ignore joypad 1 for now
                0
            }

            0x4017 => {
                //ignore joypad 2 for now
                0
            }

            //todo 0x4000 - 0x8000
            PRG_ROM..=PRG_ROM_END => self.read_prg_rom(pos),
            _ => {
                unimplemented!("attempting to read from {:x}", pos);
            }
        }
    }

    pub fn tick(&mut self, cycles: u16) {
        self.cycles += cycles as usize;
        let mut ppu = self.ppu.borrow_mut();
        ppu.tick(cycles * 3); //todo: oh my..
        self.nmi_interrupt = ppu.poll_nmi_interrupt();
    }

    fn read_prg_rom(&self, mut pos: u16) -> u8 {
        //todo: mapper
        pos -= 0x8000;
        if self.rom.prg_rom.len() == 0x4000 && pos > 0x4000 {
            //mirror if needed
            pos -= 0x4000;
        }
        self.rom.prg_rom[pos as usize]
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }
}

pub trait CpuBus: Mem {
    fn poll_nmi_status(&mut self) -> Option<u8>;
    fn tick(&mut self, cycles: u8);
}

impl Mem for Bus<NesPPU> {
    fn write(&mut self, pos: u16, data: u8) {
        Bus::write(self, pos, data);
    }

    fn read(&self, pos: u16) -> u8 {
        Bus::read(self, pos)
    }
}

impl CpuBus for Bus<NesPPU> {
    fn poll_nmi_status(&mut self) -> Option<u8> {
        Bus::poll_nmi_status(self)
    }

    fn tick(&mut self, cycles: u8) {
        Bus::<NesPPU>::tick(self, cycles as u16);
    }
}

pub struct DynamicBusWrapper {
    bus: Rc<RefCell<dyn CpuBus>>,
}

impl DynamicBusWrapper {
    pub fn new(data: Rc<RefCell<dyn CpuBus>>) -> Self {
        DynamicBusWrapper { bus: data }
    }
}

impl Mem for DynamicBusWrapper {
    fn write(&mut self, pos: u16, data: u8) {
        self.bus.borrow_mut().write(pos, data);
    }

    fn write_u16(&mut self, pos: u16, data: u16) {
        self.bus.borrow_mut().write_u16(pos, data);
    }
    fn read(&self, pos: u16) -> u8 {
        self.bus.borrow().read(pos)
    }
    fn read_u16(&self, pos: u16) -> u16 {
        self.bus.borrow().read_u16(pos)
    }
}

impl CpuBus for DynamicBusWrapper {
    fn poll_nmi_status(&mut self) -> std::option::Option<u8> {
        self.bus.borrow_mut().poll_nmi_status()
    }

    fn tick(&mut self, cycles: u8) {
        self.bus.borrow_mut().tick(cycles);
    }
}

pub struct MockBus {
    pub space: [u8; 0x10000],
    pub nmi_interrupt: Option<u8>,
    pub cycles: usize,
}

impl Mem for MockBus {
    fn write(&mut self, pos: u16, data: u8) {
        self.space[pos as usize] = data
    }

    fn read(&self, pos: u16) -> u8 {
        self.space[pos as usize]
    }
}

impl CpuBus for MockBus {
    fn poll_nmi_status(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
    }
}

impl MockBus {
    pub fn new() -> Self {
        MockBus {
            space: [0; 0x10000],
            nmi_interrupt: None,
            cycles: 0,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ppu::ppu::test;
    use crate::ppu::ppu::test::MockPPU;
    use crate::rom::ines::test_ines_rom;

    fn stub_bus() -> Bus<MockPPU> {
        Bus {
            ram: [0; 0x800],
            rom: test_ines_rom::test_rom(),
            nmi_interrupt: None,
            cycles: 0,
            ppu: RefCell::from(test::stub_ppu()),
        }
    }

    #[test]
    fn test_ram_mirrors() {
        let mut bus = stub_bus();

        bus.write(0x1005, 0x66);
        assert_eq!(bus.read(0x0005), 0x66);
        assert_eq!(bus.read(0x0805), 0x66);
        assert_eq!(bus.read(0x1805), 0x66);

        bus.write(0x1805, 0x55);
        assert_eq!(bus.read(0x0005), 0x55);
        assert_eq!(bus.read(0x0805), 0x55);
        assert_eq!(bus.read(0x1005), 0x55);
    }

    #[test]
    fn test_ppu_register_mirrors() {
        let mut bus = stub_bus();

        bus.write(0x2008, 1);
        assert_eq!(bus.ppu.borrow().ctrl, 1);

        // from: https://wiki.nesdev.com/w/index.php/PPU_registers
        //a write to $3456 is the same as a write to $2006.
        bus.write(0x3456, 5);
        assert_eq!(bus.ppu.borrow().addr, 5);
    }

    #[test]
    fn test_write_to_0x4014_oam_dma() {
        let mut bus = stub_bus();
        let base = 0x0800;
        let mut expected_result: [u8; 256] = [0; 256];
        for i in 0..255u8 {
            bus.write(base + i as u16, i);
            expected_result[i as usize] = i;
        }

        bus.write(0x4014, 0x08);

        assert_eq!(bus.cycles, 513);

        assert!(
            bus.ppu
                .borrow()
                .oam
                .iter()
                .zip(expected_result.iter())
                .all(|(a, b)| a == b),
            "oam data arrrays are not equal"
        );
    }
}
