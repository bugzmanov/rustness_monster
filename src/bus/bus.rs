use crate::cpu::cpu::Mem;
use crate::rom::ines::Rom;

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

pub struct Bus {
    pub ram: [u8; 0x800],
    pub rom: Rom,
}

#[allow(dead_code)]
impl Bus {
    const ZERO_PAGE: u16 = 0x0;
    const STACK: u16 = 0x0100;
    const RAM: u16 = 0x0200;
    const RAM_MIRRORS: u16 = 0x0800;
    const RAM_MIRRORS_END: u16 = 0x1FFF;
    const IO_REGISTERS: u16 = 0x2000;
    const IO_MIRRORS: u16 = 0x2008;
    const IO_MIRRORS_END: u16 = 0x3FFF;
    const PRG_ROM: u16 = 0x8000;

    fn map_mirrors(pos: u16) -> u16 {
        match pos {
            Bus::RAM_MIRRORS..=Bus::RAM_MIRRORS_END => pos & 0b11111111111,
            Bus::IO_MIRRORS..=Bus::IO_MIRRORS_END => pos & 0b10000000000111,
            _ => pos,
        }
    }

    pub fn write(&mut self, pos: u16, data: u8) {
        let pos = Bus::map_mirrors(pos);

        if pos < Bus::RAM_MIRRORS {
            self.ram[pos as usize] = data;
        } else if pos >= Bus::PRG_ROM {
            panic!("attempt to write to ROM"); //sram?
        } else {
            //todo
        }
    }

    pub fn read(&self, pos: u16) -> u8 {
        let pos = Bus::map_mirrors(pos);

        if pos < Bus::RAM_MIRRORS {
            self.ram[pos as usize]
        } else if pos == 0x2002 {
            0b10000000
        } else if pos >= Bus::PRG_ROM {
            self.read_prg_rom(pos)
        } else {
            0 //todo
        }
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
}

impl Mem for Bus {
    fn write(&mut self, pos: u16, data: u8) {
        Bus::write(self, pos, data);
    }

    fn write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write(pos, lo);
        self.write(pos + 1, hi);
    }

    fn read(&self, pos: u16) -> u8 {
        Bus::read(self, pos)
    }

    fn read_u16(&self, pos: u16) -> u16 {
        let lo = self.read(pos) as u16;
        let hi = self.read(pos + 1) as u16;
        (hi  << 8) | (lo as u16)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::rom::ines::test_ines_rom;

    fn test_bus() -> Bus {
        Bus {
            ram: [0; 0x800],
            rom: test_ines_rom::test_rom(),
        }
    }

    #[test]
    fn test_ram_mirrors() {
        let bus: &mut dyn Mem = &mut test_bus();

        bus.write(0x1005, 0x66);
        assert_eq!(bus.read(0x0005), 0x66);
        assert_eq!(bus.read(0x0805), 0x66);
        assert_eq!(bus.read(0x1805), 0x66);

        bus.write(0x1805, 0x55);
        assert_eq!(bus.read(0x0005), 0x55);
        assert_eq!(bus.read(0x0805), 0x55);
        assert_eq!(bus.read(0x1005), 0x55);
    }
}
