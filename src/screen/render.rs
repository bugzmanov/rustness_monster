use super::frame::Frame;
use super::pallete;
use crate::ppu::ppu::NesPPU;
use crate::rom::Mirroring;

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
    let attr_byte = ppu.vram[vram_idx];

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

    render_background(ppu, &mut frame);
    render_sprites(ppu, &mut frame);
    frame
}

fn render_background(ppu: &NesPPU, frame: &mut Frame) {
    let scroll_x = (ppu.scroll.scroll_x) as i32;
    let scroll_y = (ppu.scroll.scroll_y) as i32;
    // println!("{} {}" ,scroll_x, scroll_y);
    // println!("{:x}", ppu.ctrl.nametable_addr());

    for i in 0..(0x3c0) {
        let (tile_addr, tile_bg_addr) = tile_vram_addr(i, ppu);
        let (tile, palette) = load_tile_and_palette(ppu, tile_addr, tile_bg_addr);

        let delta_y = (scroll_y % 8) as isize;
        let delta_x = (scroll_x % 8) as isize;

        let tile_x = (i % 32) as isize;
        let tile_y = (i / 32) as isize;
        render_tile(
            tile_x * 8 - delta_x,
            tile_y * 8 - delta_y,
            tile,
            palette,
            ppu,
            frame,
        );
    }

    //a hack to fill up right-most space:
    // scroll shifts the whole picture to the left and leave upto 8 pixels wide black space on the right
    if scroll_x % 8 != 0 {
        for i in 0..30 {
            let x = (255 - scroll_x % 8) as isize;
            let y = (i * 8) as isize;

            let (_, tile_bg_addr) = tile_vram_addr(i * 32, ppu);

            let mut tile_addr = (scroll_x / 8 + i as i32 * 32) as u16;

            if ppu.ctrl.nametable_addr() == 0x2000 {
                tile_addr += 0x2400;
            } else {
                tile_addr += 0x2000;
            }

            let (tile, palette) = load_tile_and_palette(ppu, tile_addr, tile_bg_addr);

            render_tile(x + 1, y, tile, palette, ppu, frame);
        }
    }
}

fn render_tile(x: isize, y: isize, tile: &[u8], palette: [u8; 4], ppu: &NesPPU, frame: &mut Frame) {
    for dy in 0..=7 {
        let mut upper = tile[dy];
        let mut lower = tile[dy + 8];

        for dx in (0..=7).rev() {
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
            let pixel_x = x + dx as isize;
            let pixel_y = y + dy as isize;
            if pixel_x >= 0 && pixel_x < 256 && pixel_y >= 0 && pixel_y < 240 {
                frame.set_pixel((pixel_x as i32) as usize, (pixel_y as i32) as usize, rgb);
            }
        }
    }
}

//dark magic to support scrolling :(
fn load_tile_and_palette(ppu: &NesPPU, tile_addr: u16, tile_bg_addr: u16) -> (&[u8], [u8; 4]) {
    let bank = ppu.ctrl.bknd_pattern_addr() as usize;

    let mirror_i = ppu.mirror_vram_addr(tile_addr as u16) as usize;
    let tile = ppu.vram[mirror_i] as usize;

    let mirror_i2 = ppu.mirror_vram_addr(tile_bg_addr as u16) as usize;
    let test_tile_x = ((mirror_i2) % 32) as usize;
    let test_tile_y = ((mirror_i2) / 32) as usize;

    let tile = &ppu.chr_rom[(bank + tile * 16) as usize..=(bank + tile * 16 + 15) as usize];

    let palette = bg_pallette(ppu, tile_addr as u16, test_tile_x, test_tile_y);

    (tile, palette)
}

//dark magic to support scrolling :(
fn tile_vram_addr(i: usize, ppu: &NesPPU) -> (u16, u16) {
    let scroll_x = (ppu.scroll.scroll_x) as i32;
    let scroll_y = (ppu.scroll.scroll_y) as i32;

    let mut tile_addr = i as u16;
    #[allow(unused_assignments)]
    let mut tile_bg_addr = 0;

    //vertical scroll
    if let Mirroring::HORIZONTAL = ppu.mirroring {
        tile_addr += (scroll_y / 8 * 8 * 4) as u16;
        if tile_addr >= 0x3c0 {
            tile_addr += 64; //skip attribute table
        }
        tile_addr += ppu.ctrl.nametable_addr();

        tile_bg_addr = tile_addr;

        if tile_addr >= 0x2400 && tile_addr <= 0x27ff {
            //second to 3rd
            tile_addr += 0x400;
            tile_bg_addr -= 0x400;
        }
        if tile_addr >= 0x2c00 {
            // fourth to 1st
            tile_addr -= 3 * 0x400;
            tile_bg_addr -= 3 * 0x400;
        }

        if ppu.ctrl.nametable_addr() == 0x2800 && tile_addr >= 0x2800 && tile_addr <= 0x2BFF {
            tile_bg_addr -= 0x400;
        }
    } else {
        //horizontal scroll
        if (i % 32 as usize + (ppu.scroll.scroll_x / 8) as usize) > 31 {
            tile_addr += 0x400;
            tile_addr -= 32;
        }
        tile_addr += (scroll_x / 8) as u16;

        if tile_addr >= 0x3c0 && tile_addr < 0x3c0 + 64 {
            tile_addr += 64; //skip attribute table
        }
        tile_addr += ppu.ctrl.nametable_addr();

        tile_bg_addr = tile_addr;

        if tile_bg_addr >= ppu.ctrl.nametable_addr() + 1024 {
            if ppu.ctrl.nametable_addr() == 0x2000 {
                tile_bg_addr -= 0x400;
            } else {
                tile_bg_addr -= 0x800;
            }
        } else {
            if ppu.ctrl.nametable_addr() == 0x2400 {
                tile_bg_addr -= 0x400;
            }
        }
    }
    (tile_addr, tile_bg_addr)
}

fn render_sprites(ppu: &NesPPU, frame: &mut Frame) {
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
}
