use crate::cpu::cpu::Mem;
use crate::cpu::cpu::CPU; 
use crate::nes::ines::Rom;

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
const ZERO_PAGE: u16 = 0x0;
const STACK: u16 = 0x0100;
const RAM: u16 = 0x0200;
const RAM_MIRRORS: u16 = 0x0800;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const IO_REGISTERS: u16 = 0x2000;
const IO_MIRRORS: u16 = 0x2008;
const IO_MIRRORS_END: u16 = 0x3FFF;
const PRG_ROM: u16 = 0x8000;

pub struct Bus {
    pub ram: [u8;0x800],
    pub rom: Rom,
    pub cpu: CPU,
}

impl Bus {
    fn map_mirrors(pos: u16) -> u16 {
        match pos {
            RAM_MIRRORS ..=RAM_MIRRORS_END => pos & 0b11111111111,
            IO_MIRRORS ..=IO_MIRRORS_END => pos & 0b10000000000111,
           _ => pos, 
        }
    }
    
    pub fn write(&mut self, pos: u16, data: u8) {
        let pos = Bus::map_mirrors(pos);

        if pos < RAM_MIRRORS {
            self.ram[pos as usize] = data;
        } else if pos >= PRG_ROM {
            panic!("attempt to write to ROM"); //sram?
        } else {
            //todo
        }
    }

    // pub fn write_u16(&mut self, pos: u16, data: u16) {
    //     let pos = Bus::map_mirrors(pos);

    //     if pos < RAM_MIRRORS {
    //         LittleEndian::write_u16(&mut self.ram[pos as usize..], data)
    //     } else if pos >= PRG_ROM {
    //         panic!("writing to ROM");
    //     }
    //     } else {
    //         0
    //     }
    // }

    pub fn read(&self, pos: u16) -> u8 {
        let pos = Bus::map_mirrors(pos);

        if pos < RAM_MIRRORS {
            self.ram[pos as usize]
        } else if pos >= PRG_ROM {
            self.read_prg_rom(pos)
        } else {
            0 //todo
        }
    }

    fn read_prg_rom(&self, pos: u16) -> u8 {
        //todo: mapper
        self.rom.prg_rom[pos as usize]
    }
    // pub fn read_u16(&self, pos: u16) -> u16 { // check baundries?
    //     let pos = Bus::map_mirrors(pos);

    //     if pos < RAM_MIRRORS {
    //         LittleEndian::read_u16(&self.ram[pos as usize..])
    //     } else {
    //         0 //todo
    //     }
    // }


}


impl Mem for Bus {
    fn write(&mut self, pos: u16, data: u8)  {
        self.write(pos, data);
    }

    fn write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write(pos, lo);
        self.write(pos+1, hi);
    }

    fn read(&self, pos: u16) -> u8 {
        self.read(pos)
    }

    fn read_u16(&self, pos: u16) -> u16 {
        let lo = self.read(pos);
        let hi = self.read(pos+1);
        ((hi << 8) as u16) | (lo as u16)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::nes::ines::test_ines_rom;

    fn test_bus() -> Bus {
        Bus {
            cpu: CPU::new(),
            ram: [0; 0x800],
            rom: test_ines_rom::test_rom(),
        }
    }

    #[test]
    fn test_ram_mirrors() {
        let bus: &mut Mem = &mut test_bus();

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