// http://www.dustmop.io/blog/2015/04/28/nes-graphics-part-1/

use crate::ppu::registers::control::ControlRegister;
use crate::ppu::registers::mask::MaskRegister;
use crate::ppu::registers::status::StatusRegister;
use crate::rom::ines::Mirroring;
use crate::screen::frame::Frame;
use crate::screen::pallete;

pub struct NesPPU {
    chr_rom: Vec<u8>,
    mirroring: Mirroring,
    ctrl: ControlRegister,
    mask: MaskRegister,
    status: StatusRegister,
    oam_addr: u8,
    scroll: Scroll,
    addr: Addr,
    vram: [u8; 2048],
    oam_data: [u8; 256],
    pub line: usize,
    pub cycles: usize,
    nmi_interrupt: Option<u8>,
    palette_table: [u8; 32],
    read_data_buf: u8,
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

struct Scroll {
    pub scroll_x: u8,
    pub scroll_y: u8,
    latch: bool
}

impl Scroll {

    fn new() -> Self {
        Scroll {
            scroll_x: 0,
            scroll_y: 0,
            latch: false
        }
    }

    fn write(&mut self, data: u8) {
        if !self.latch {
            self.scroll_x = data;
        } else {
            self.scroll_y = data;
        }
        self.latch = !self.latch;
    }

    fn reset_latch(&mut self) {
        self.latch = false;
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
    fn tick(&mut self, cycles: u16) -> bool;
    fn poll_nmi_interrupt(&mut self) -> Option<u8>;
}

pub trait Renderer {
    fn render(ppu: &NesPPU) -> Frame;
}

// https://wiki.nesdev.com/w/index.php/PPU_attribute_tables
fn sprite_palette(ppu: &NesPPU, pallete_idx: u8) -> [u8; 4] {
    let start = 0x11 + (pallete_idx * 4) as usize;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

fn bg_pallette(ppu: &NesPPU, tile_addr: u16, tile_x: usize, tile_y: usize) -> [u8; 4] {
    let attr_table_idx = tile_y / 4 * 8 + tile_x / 4;

    let pos = match tile_addr {
        0x2000..=0x23FF => 0x23C0,
        0x2400..=0x27FF => 0x27C0,
        0x2800..=0x2BFF => 0x2BC0,
        0x2C00..=0x2FFF => 0x2FC0,
        0x3000..=0x3FFF => return bg_pallette(ppu, tile_addr & 0b10111111111111, tile_x, tile_y),
        _ => panic!("unreachable addr {:x}", tile_addr),
    };

    // println!("x:{},y:{}, start:{:x} pos:{:x}", tile_x, tile_y, tile_addr, pos);
    let vram_idx = ppu.mirror_vram_addr((pos + attr_table_idx) as u16) as usize;
    let attr_byte = ppu.vram[vram_idx ];

    let pallet_idx = match (tile_x % 4 / 2, tile_y % 4 / 2) {
        (0, 0) => attr_byte & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        (_, _) => panic!("should not happen"),
    };

    let pallete_start: usize = 1 + (pallet_idx as usize) * 4;
    [
        ppu.palette_table[0],
        ppu.palette_table[pallete_start],
        ppu.palette_table[pallete_start + 1],
        ppu.palette_table[pallete_start + 2],
    ]
}


pub fn render(ppu: &NesPPU) -> Frame {
    let mut frame = Frame::new();
    let bank = ppu.ctrl.bknd_pattern_addr();
    let scroll_x = (ppu.scroll.scroll_x ) as i32;
    let scroll_y = (ppu.scroll.scroll_y ) as i32;

    println!("{} {}" ,scroll_x, scroll_y);
    println!("{:x}", ppu.ctrl.nametable_addr());
    // for i in 0..0x3c0 {
    for i in 0..0x3c0 {
        
        let mut start = i as u16; //(offset_x as u16)+ ((offset_y * 4) as u16);
        // if offset_y % 8 == 0 {
            start += ((scroll_x /8 *8 * 4) as u16);
            start += ((scroll_y /8 *8 * 4) as u16);
            if start >= 0x3c0 {
                start += 64; //skip attribute table
            }
        // }
         
  
        start += ppu.ctrl.nametable_addr();


        if(start >= 0x3000) {
            panic!("wtf");
        }    

        let mut start2 = start;
        if(start >= 0x2400 && start <= 0x27ff) { //second to 3rd
            start += (0x400);
            start2 -= 0x400;
        }
        if(start >= 0x2c00 ) { // fourth to 1st
            start -= (3*0x400);
            //start2 -= (3*0x400 - 0x200);
            start2 -= (3*0x400);
        }


        // let mirror_i = ppu.mirror_vram_addr(i as u16) as usize; 
        let mirror_i = ppu.mirror_vram_addr(start as u16) as usize; 
        // println!("{:x}={}", start, mirror_i);
        let tile = ppu.vram[mirror_i] as u16;
        

        // println!("{} {}", i, mirror_i);
        let tile_x = i % 32 as usize;
        let tile_y = ((i / 32) as usize);

        // let test_tile_x = ((i as u16 +  ((scroll_x /8 *8 * 4) as u16)) % 32) as usize ;

        // let test_tile_y = ((i as u16 +  ((scroll_y /8 *8 * 4) as u16)) / 32) as usize ;
        // let mirror_i2 = ppu.mirror_vram_addr(start2 as u16) as usize; 
        let mirror_i2 = ppu.mirror_vram_addr(start2 as u16) as usize; 

        let test_tile_x = ((mirror_i2) % 32) as usize ;
        let test_tile_y = ((mirror_i2) / 32) as usize ;

        // println!("i: {} tile:{} delta:{}", i, tile_y, delta_y);
        // tile_y -= delta_y;
        // if(tile != 32 && tile != 0) {
        //     println!("{}:x={},y={}", tile, tile_x, tile_y);
        // }
        let tile = &ppu.chr_rom[(bank + tile * 16) as usize..=(bank + tile * 16 + 15) as usize];

        // let palette = bg_pallette(ppu, start as u16, test_tile_x, test_tile_y);
        let palette = bg_pallette(ppu, start as u16, test_tile_x, test_tile_y);
        let delta_y = (scroll_y % 8) as usize; 
        let delta_x = (scroll_x % 8) as usize; 

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => pallete::YUV[ppu.palette_table[0] as usize],
                    1 => pallete::YUV[palette[1] as usize],
                    2 => pallete::YUV[palette[2] as usize],
                    3 => pallete::YUV[palette[3] as usize],
                    _ => panic!("can't be"),
                };
                let pixel_x = (tile_x * 8 + x).saturating_sub(delta_x);
                let pixel_y = (tile_y * 8 + y).saturating_sub(delta_y);
                frame.set_pixel((pixel_x as i32 ) as usize, (pixel_y as i32 ) as usize, rgb)
            }
        }
    }

    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        let flip_vertical = if ppu.oam_data[i + 2] >> 7 & 1 == 1 {
            true
        } else {
            false
        };
        let flip_horizontal = if ppu.oam_data[i + 2] >> 6 & 1 == 1 {
            true
        } else {
            false
        };
        let pallette_idx = ppu.oam_data[i + 2] & 0b11;
        let sprite_palette = sprite_palette(ppu, pallette_idx);
        let bank: u16 = ppu.ctrl.sprt_pattern_addr();
        let tile = ppu.oam_data[i + 1] as u16;
        let tile_x = ppu.oam_data[i + 3] as usize;
        let tile_y = ppu.oam_data[i] as usize;
        let tile = &ppu.chr_rom[(bank + tile * 16) as usize..=(bank + tile * 16 + 15) as usize];


        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            'ololo: for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => continue 'ololo, //pallete::YUV[0x01],
                    1 => pallete::YUV[sprite_palette[1] as usize],
                    2 => pallete::YUV[sprite_palette[2] as usize],
                    3 => pallete::YUV[sprite_palette[3] as usize],
                    _ => panic!("can't be"),
                };
                match (flip_horizontal, flip_vertical) {
                    (false, false) => frame.set_pixel(tile_x + x, tile_y + y, rgb),
                    (true, false) => frame.set_pixel(tile_x + 7 - x, tile_y + y, rgb),
                    (false, true) => frame.set_pixel(tile_x + x, tile_y + 7 - y, rgb),
                    (true, true) => frame.set_pixel(tile_x + 7 - x, tile_y + 7 - y, rgb),
                }
            }
        }
    }
    frame
}

impl NesPPU {
    pub fn new_empty_rom() -> Self {
        NesPPU::new(vec![0; 2048], Mirroring::HORIZONTAL)
    }

    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        NesPPU {
            chr_rom: chr_rom,
            mirroring: Mirroring::HORIZONTAL,//mirroring,
            ctrl: ControlRegister::new(),
            mask: MaskRegister::new(),
            status: StatusRegister::new(),
            oam_addr: 0,
            scroll: Scroll::new(),
            addr: Addr::new(),
            vram: [0; 2048],
            oam_data: [0; 64 * 4],
            line: 0,
            cycles: 0,
            nmi_interrupt: None,
            palette_table: [0; 32],
            read_data_buf: 0,
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

    fn has_sprite_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        // (y == self.line) && self.registers.is_sprite_enable()
        (y == self.line) && x <= cycle && self.mask.show_sprites()
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
        self.scroll.reset_latch();
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

    fn write_to_scroll(&mut self, value: u8) {
        self.scroll.write(value);
    }

    fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.udpate(value);
        if self.addr.read() > 0x3fff {
            self.addr.set(self.addr.read() & 0b11111111111111); //mirror down addr above 0x3fff
        }
    }

    fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.read();
        match addr {
            0..=0x1fff => println!("attempt to write to chr rom space {}", addr), //panic!("attempt to write to chr rom space {}", addr),
            0x2000..=0x2fff => {
                // if(addr >= 0x2000 && addr < 0x23ff) {
                //      print!("{:x} ", value);
                // }
                self.vram[self.mirror_vram_addr(addr) as usize] = value;
            }
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used in reallity", addr),
            0x3f00..=0x3fff =>
            /* todo: implement working with palette */
            {
                self.palette_table[(addr - 0x3f00) as usize] = value;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }

    fn read_data(&mut self) -> u8 {
        let addr = self.addr.read();

        self.increment_vram_addr();

        match addr {
            0..=0x1fff => {
                let result = self.read_data_buf;
                self.read_data_buf = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2fff => {
                let result = self.read_data_buf;
                self.read_data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
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
    }

    fn tick(&mut self, cycles: u16) -> bool {
        self.cycles += cycles as usize;
        if self.cycles >= 341 {
            if self.has_sprite_hit(self.cycles) {
                self.status.set_sprite_zero_hit(true);
            }

            self.cycles = self.cycles - 341;
            self.line += 1;

            if self.line == 241 {
                self.status.set_vblank_status(true);
                self.status.set_sprite_zero_hit(false);
                if self.ctrl.generate_vblank_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }

            if self.line >= 262 {
                self.line = 0;
                self.nmi_interrupt = None;
                self.status.set_sprite_zero_hit(false);
                // self.status.
                self.status.reset_vblank_status();
                return true;
            }
        }
        return false;
    }

    fn poll_nmi_interrupt(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }
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
        fn tick(&mut self, cycles: u16) -> bool {
            self.ticks += cycles as usize;
            false
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

    // todo:figure out why it's writing to rom
    // #[test]
    // #[should_panic]
    // fn test_ppu_writing_to_chr_rom_is_prohibited() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_ppu_addr(0x03);
    //     ppu.write_to_ppu_addr(0x05);
    //     ppu.write_to_data(0x66);
    // }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.addr.read(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
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

        ppu.read_data(); //load_into_buffer
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

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
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

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from a

        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from B
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x63); //0x6305 -> 0x2305
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into_buffer
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
