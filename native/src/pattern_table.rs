extern crate sdl2;

use rustness::screen::frame;
use rustness::screen::pallete;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use std::time::Duration;
use std::time::SystemTime;

use rustness::rom::Rom;
use std::fs::File;
use std::io::Read;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("rust nes demo", 256 * 3, 240 * 3)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();

    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let frame = build_frame(0);
    let frame2 = build_frame(1);

    let mut pointer = &frame;

    let mut prev_time = SystemTime::now();
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();
    let mut is_frame_1 = true;
    'running: loop {
        texture.update(None, &pointer.data, 256 * 3).unwrap();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown { .. } => {
                    // break 'running
                    if is_frame_1 {
                        pointer = &frame2
                    } else {
                        pointer = &frame;
                    }
                    is_frame_1 = !is_frame_1;
                }
                _ => {}
            }
        }

        canvas.clear();
        canvas.set_scale(3.0, 3.0).unwrap();
        canvas
            .copy(&texture, None, Some(Rect::new(10, 10, 266, 250)))
            .unwrap();
        canvas.present();

        let elapsed_time = SystemTime::now()
            .duration_since(prev_time)
            .unwrap()
            .as_nanos();

        let wait = if elapsed_time < 1_000_000_000u128 / 60 {
            1_000_000_000u32 / 60 - (elapsed_time as u32)
        } else {
            0
        };
        ::std::thread::sleep(Duration::new(0, wait));
        prev_time = SystemTime::now();
    }

    // fn rom_sprite_palette(rom: &Rom) -> [(u8,u8,u8); 3] {
    // [pallete::YUV[rom.chr_rom[0x3f01] as usize ], pallete::YUV[rom.chr_rom[0x3f02] as usize ],pallete::YUV[rom.chr_rom[0x3f03] as usize]]
    // }

    fn build_frame(bank: usize) -> frame::Frame {
        let mut file = File::open("test_rom/pacman.nes").unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        let rom = Rom::load(&data).unwrap();
        let mut frame = frame::Frame::new();

        let mut tile_y = 0;
        let mut tile_x = 0;
        let bank = (bank * 0x1000) as usize;
        // let bank = 0;
        // let mut rng = rand::thread_rng();
        // let palette = rom_sprite_palette(&rom);

        for tile_n in 0..255 {
            if tile_n != 0 && tile_n % 20 == 0 {
                tile_y += 10;
                tile_x = 0;
            }
            let tile = &rom.chr_rom[(bank + tile_n * 16)..=(bank + tile_n * 16 + 15)];

            for y in 0..=7 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];

                for x in (0..=7).rev() {
                    let value = (1 & upper) << 1 | (1 & lower);
                    upper = upper >> 1;
                    lower = lower >> 1;
                    let rgb = match value {
                        0 => pallete::YUV[0x01],
                        1 => pallete::YUV[0x23],
                        2 => pallete::YUV[0x27],
                        3 => pallete::YUV[0x2b],
                        _ => panic!("can't be"),
                    };
                    frame.set_pixel(tile_x + x, tile_y + y, rgb)
                }
            }

            tile_x += 10;
        }
        frame
    }
}
