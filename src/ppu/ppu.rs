use crate::ppu::registers::control::ControlRegister;
use crate::ppu::registers::mask::MaskRegister;
use crate::ppu::registers::status::StatusRegister;
use crate::rom::ines::Mirroring;
use crate::screen::frame::Frame;

pub struct NesPPU {
    chr_rom: Vec<u8>,
    mirroring: Mirroring,
    ctrl: ControlRegister,
    mask: MaskRegister,
    status: StatusRegister,
    oam_addr: u8,
    scroll: u8,
    addr: Addr,
    oamdma: u8,
    vram: [u8; 2048],
    oam_data: [u8; 256],
    line: usize,
    cycles: usize,
    nmi_interrupt: Option<u8>,
    palette_table: [u8; 32],
}

struct Addr {
    value: (u8, u8),
    hi_ptr: bool,
}

impl Addr {
    pub fn new() -> Self {
        Addr {
            value: (0, 0), // high byte first, lo byte second
            hi_ptr: true,
        }
    }

    pub fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xff) as u8;
    }

    pub fn udpate(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }

        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        let lo = self.value.1;
        self.value.1 = self.value.1.wrapping_add(inc);
        if lo > self.value.1 {
            self.value.0 = self.value.0.wrapping_add(1);
        }
    }
    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    pub fn read(&self) -> u16 {
        ((self.value.0 as u16) << 8) | (self.value.1 as u16)
    }
}

pub trait PPU {
    fn write_to_ctrl(&mut self, value: u8);
    fn write_to_mask(&mut self, value: u8);
    fn read_status(&mut self) -> u8; //todo: this will have to be &mut
    fn write_to_oam_addr(&mut self, value: u8);
    fn write_to_oam_data(&mut self, value: u8);
    fn read_oam_data(&self) -> u8;
    fn write_to_scroll(&mut self, value: u8);
    fn write_to_ppu_addr(&mut self, value: u8);
    fn write_to_data(&mut self, value: u8);
    fn read_data(&mut self) -> u8;
    fn write_oam_dma(&mut self, value: &[u8; 256]);
    fn tick(&mut self, cycles: u16);
    fn poll_nmi_interrupt(&mut self) -> Option<u8>;
}

pub trait Renderer {
    fn render() -> Frame;
}


impl NesPPU {
    pub fn new_empty_rom() -> Self {
        NesPPU::new(vec![0; 2048], Mirroring::HORIZONTAL)
    }

    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        NesPPU {
            chr_rom: chr_rom,
            mirroring: mirroring,
            ctrl: ControlRegister::new(),
            mask: MaskRegister::new(),
            status: StatusRegister::new(),
            oam_addr: 0,
            scroll: 0,
            addr: Addr::new(),
            oamdma: 0,
            vram: [0; 2048],
            oam_data: [0; 64 * 4],
            line: 0,
            cycles: 0,
            nmi_interrupt: None,
            palette_table: [0; 32],
        }
    }

    // Horizontal:
    //   [ A ] [ a ]
    //   [ B ] [ b ]

    // Vertical:
    //   [ A ] [ B ]
    //   [ a ] [ b ]
    fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400;
        match (&self.mirroring, name_table) {
            (Mirroring::VERTICAL, 2) | (Mirroring::VERTICAL, 3) => vram_index - 0x800,
            (Mirroring::HORIZONTAL, 2) => vram_index - 0x400,
            (Mirroring::HORIZONTAL, 1) => vram_index - 0x400,
            (Mirroring::HORIZONTAL, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());

        if self.addr.read() > 0x3fff {
            //todo: fix copy-paste
            self.addr.set(self.addr.read() & 0b11111111111111); //mirror down addr above 0x3fff
        }
    }
}

impl PPU for NesPPU {
    fn write_to_ctrl(&mut self, value: u8) {
        let before_nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);
        if !before_nmi_status && self.ctrl.generate_vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(1);
        }
    }

    fn write_to_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    fn read_status(&mut self) -> u8 {
        let data = self.status.snapshot();
        self.status.reset_vblank_status();
        self.addr.reset_latch();
        data
    }

    fn write_to_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    fn write_to_scroll(&mut self, value: u8) {}

    fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.udpate(value);
        if self.addr.read() > 0x3fff {
            self.addr.set(self.addr.read() & 0b11111111111111); //mirror down addr above 0x3fff
        }
    }

    fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.read();
        match addr {
            0..=0x1fff => panic!("attempt to write to chr rom space {}", addr),
            0x2000..=0x2fff => self.vram[self.mirror_vram_addr(addr) as usize] = value,
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used in reallity", addr),
            0x3f00..=0x3fff =>
            /* todo: implement working with palette */
            {
                self.palette_table[(addr - 0x3f00) as usize] = value;
                println!("write palette {:x} {:x}", addr, value);
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }

    fn read_data(&mut self) -> u8 {
        let addr = self.addr.read();

        self.increment_vram_addr();

        match addr {
            0..=0x1fff => self.chr_rom[addr as usize],
            0x2000..=0x2fff => self.vram[self.mirror_vram_addr(addr) as usize],
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used in reallity", addr),
            0x3f00..=0x3fff =>
            /* todo: implement working with palette */
            {
                println!("read palette");
                self.palette_table[(addr - 0x3f00) as usize]
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }

    fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
        // println!("write to oam dma");
    }

    fn tick(&mut self, cycles: u16) {
        self.cycles += cycles as usize;
        // println!("{}: {}", self.line, self.cycles);
        if self.cycles > 341 {
            self.cycles = self.cycles % 341;
            self.line += 1;

            if self.line == 241 && self.cycles > 0 {
                self.status.set_vblank_status(true);

                if self.ctrl.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }

            if self.line == 261 {
                self.line = 0; //todo -1 actually
                self.status.reset_vblank_status();
            }
        }
    }

    fn poll_nmi_interrupt(&mut self) -> Option<u8> {
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
        fn read_status(&mut self) -> u8 {
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
        fn read_data(&mut self) -> u8 {
            self.data
        }
        fn write_oam_dma(&mut self, value: &[u8; 256]) {
            self.oam = value.clone();
        }
        fn tick(&mut self, cycles: u16) {
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
            vram: [0; 2048],
            oam: [0; 64 * 4],
            ticks: 0,
            line: 0,
        }
    }

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);
        ppu.write_to_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    #[should_panic]
    fn test_ppu_writing_to_chr_rom_is_prohibited() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ppu_addr(0x03);
        ppu.write_to_ppu_addr(0x05);
        ppu.write_to_data(0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.addr.read(), 0x2306)
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x66); //write to a

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x77); //write to B

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        assert_eq!(ppu.read_data(), 0x66); //read from A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        assert_eq!(ppu.read_data(), 0x77); //read from b
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = NesPPU::new(vec![0; 2048], Mirroring::VERTICAL);

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x66); //write to A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x77); //write to b

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        assert_eq!(ppu.read_data(), 0x66); //read from a

        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        assert_eq!(ppu.read_data(), 0x77); //read from B
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x63); //0x6305 -> 0x2305
        ppu.write_to_ppu_addr(0x05);

        assert_eq!(ppu.read_data(), 0x66);
        // assert_eq!(ppu.addr.read(), 0x0306)
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_oam_addr(0x10);
        ppu.write_to_oam_data(0x66);
        ppu.write_to_oam_data(0x77);

        ppu.write_to_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_to_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut ppu = NesPPU::new_empty_rom();

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_to_oam_addr(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_to_oam_addr(0xf); //wrap around
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_to_oam_addr(0x10);
        ppu.write_to_oam_addr(0x77);
        ppu.write_to_oam_addr(0x11);
        ppu.write_to_oam_addr(0x66);
    }
}
