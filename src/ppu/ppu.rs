use crate::ppu::registers::status::StatusRegister;
use crate::ppu::registers::mask::MaskRegister;
use crate::ppu::registers::control::ControlRegister;
use std::cell::RefCell;

pub struct NesPPU {
    ctrl: ControlRegister,
    mask: MaskRegister,
    status: RefCell<StatusRegister>,
    oamaddr: u8,
    oamdata: u8,
    scroll: u8,
    addr: u8,
    data: u8,
    oamdma: u8,
    vram: [u8; 2048],
    oam: [u8; 64 * 4],

    line: usize,
    cycles:usize,
    nmi_interrupt: Option<u8>,
}

pub trait PPU {
    fn write_to_ctrl(&mut self, value: u8);
    fn write_to_mask(&mut self, value: u8);
    fn read_status(&self) -> u8; //todo: this will have to be &mut
    fn write_to_oam_addr(&mut self, value: u8);
    fn write_to_oam_data(&mut self, value: u8);
    fn read_oam_data(&self) -> u8;
    fn write_to_scroll(&mut self, value: u8);
    fn write_to_ppu_addr(&mut self, value: u8);
    fn write_to_data(&mut self, value: u8);
    fn read_data(&self) -> u8;
    fn write_to_oam_dma(&mut self, value: u8);
    fn tick(&mut self, cycles: u8);
    fn poll_nmi_interrupt(& mut self) -> Option<u8>;
}

impl NesPPU {
    pub fn new() -> Self {
        NesPPU {
            ctrl: ControlRegister::new(),
            mask: MaskRegister::new(),
            status: RefCell::from(StatusRegister::new()),
            oamaddr: 0,
            oamdata: 0,
            scroll: 0,
            addr: 0,
            data: 0,
            oamdma: 0,
            vram: [0; 2048],
            oam: [0; 64 * 4],
            line: 0,
            cycles: 0,
            nmi_interrupt: None
        }
    }
}

impl PPU for NesPPU {
    fn write_to_ctrl(&mut self, value: u8) {
        let before_nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);
        if(!before_nmi_status && self.ctrl.generate_vblank_nmi() && self.status.borrow().is_in_vblank()) {
            self.nmi_interrupt = Some(1);
        }
    }

    fn write_to_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    fn read_status(&self) -> u8 {
        let data = self.status.borrow().snapshot();
        self.status.borrow_mut().reset_vblank_status();
        if data != 0 {
            println!("{}", data);
        }
        //todo: this will have to be &mut
        data
    }

    fn write_to_oam_addr(&mut self, value: u8) {}

    fn write_to_oam_data(&mut self, value: u8) {
    }

    fn read_oam_data(&self) -> u8 {
        0
    }

    fn write_to_scroll(&mut self, value: u8) {}

    fn write_to_ppu_addr(&mut self, value: u8) {}

    fn write_to_data(&mut self, value: u8) {}

    fn read_data(&self) -> u8 {
        0
    }

    fn write_to_oam_dma(&mut self, value: u8) {}

    fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        // println!("{}: {}", self.line, self.cycles);
        if self.cycles > 341 {
            self.cycles = self.cycles % 341;
            self.line += 1;
            
            if self.line == 241 && self.cycles > 0 {
                self.status.borrow_mut().set_vblank_status(true);
                
                if self.ctrl.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }

            if self.line == 261 {
                self.line = 0; //todo -1 actually
                self.status.borrow_mut().reset_vblank_status();
            }
        }

    }

    fn poll_nmi_interrupt(& mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    // fn build_bg_screen(&self, chr_rom: &[u8]) -> Frame {
    //     let mut frame = Frame::new();

    //     for y in 0..29 {
    //         for x in 0..31 {
    //             let tile_n = self.vram[y * 32 + x] as usize;
    //             let bank = ((self.crtl as usize >> 4) & 1) * 0x1000;
    //             let tile = &chr_rom[(bank + tile_n * 16)..(bank + tile_n * 16 + 15)];

    //             let memtable_attr_pos = 0x3C0 + self.line / 32 * 8;
    //         }
    //     }

    //     frame
    // }

    // fn build_line(&mut self) {
    //     // self.line += 1;
    //     let nametamble_pos = (self.line /8 * 32);
    //     let memtable_slice:&[u8] = &self.vram[nametamble_pos .. (nametamble_pos + 32)];

    //     let memtable_attr_pos = 0x3C0 + self.line / 32 * 8;

    //     let memtable_attr_slice: &[u8] = &self.vram[memtable_attr_pos .. (memtable_attr_pos +8)];

    //     self.line += 1;

    // }
}

#[cfg(test)]
pub mod test {
    use super::*;
    pub struct MockPPU {
        pub ctrl: u8,
        pub mask: u8,
        pub status: u8,
        pub oamaddr: u8,
        pub oamdata: u8,
        pub scroll: u8,
        pub addr: u8,
        pub data: u8,
        pub oamdma: u8,
        pub vram: [u8; 2048],
        pub oam: [u8; 64 * 4],
        pub ticks: usize,
        line: usize,
    }

    impl PPU for MockPPU {
        fn write_to_ctrl(&mut self, value: u8) {
            self.ctrl = value;
        }
        fn write_to_mask(&mut self, value: u8) {
            self.mask = value;
        }
        fn read_status(&self) -> u8 {
            self.status
        }
        fn write_to_oam_addr(&mut self, value: u8) {
            self.oamaddr = value;
        }
        fn write_to_oam_data(&mut self, value: u8) {
            self.oamdata = value;
        }
        fn read_oam_data(&self) -> u8 {
            self.oamdata
        }
        fn write_to_scroll(&mut self, value: u8) {
            self.scroll = value;
        }
        fn write_to_ppu_addr(&mut self, value: u8) {
            self.addr = value;
        }
        fn write_to_data(&mut self, value: u8) {
            self.data = value;
        }
        fn read_data(&self) -> u8 {
            self.data
        }
        fn write_to_oam_dma(&mut self, value: u8) {
            self.oamdma = value
        }
        fn tick(&mut self, cycles: u8) {
            self.ticks += cycles as usize;
        }
        fn poll_nmi_interrupt(&mut self) -> Option<u8> {
            None
        }
    }

    pub fn stub_ppu() -> MockPPU {
        MockPPU {
            ctrl: 0,
            mask: 0,
            status: 0,
            oamaddr: 0,
            oamdata: 0,
            scroll: 0,
            addr: 0,
            data: 0,
            oamdma: 0,
            vram: [0; 2048],
            oam: [0; 64 * 4],
            ticks: 0,
            line: 0,
        }
    }
}
