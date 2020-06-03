use rustness::bus::bus::Bus;
use rustness::cpu::cpu::CPU;
use rustness::ppu::ppu;
use rustness::ppu::ppu::NesPPU;
use rustness::rom::ines::Rom;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use std::time::Duration;
use std::time::SystemTime;

use std::fs::File;
use std::io::Read;


use std::cell::RefCell;
use std::{rc::Rc};


fn main() {
    // let mut file = File::open("test_rom/ice_climber.nes").unwrap();
    let mut file = File::open("test_rom/pacman.nes").unwrap();
    // let mut file = File::open("test_rom/nestest.nes").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let rom = Rom::load(&data).unwrap();

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

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    canvas.set_scale(3.0, 3.0).unwrap();
    let mut prev_time = SystemTime::now();

    let trace = Rc::from(RefCell::from(false));

    let trace_rc = trace.clone();
    let func = move |z: &NesPPU| {

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                    let upd = !*trace_rc.borrow();
                    trace_rc.replace(upd);
                }
                _ => {}
            }
        }

        let frame = ppu::render(z);
        texture.update(None, &frame.data, 256 * 3).unwrap();
        canvas.clear();

        canvas
            .copy(&texture, None, Some(Rect::new(0, 0, 256, 240)))
            .unwrap();
        canvas.set_scale(3.0, 3.0).unwrap();
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
    };

    let mut bus = Bus::<'_, NesPPU>::new(rom, func);

    let pc = bus.read(0xfffc);
    let ffd = bus.read(0xfffd);

    // let memory = Rc::from(RefCell::from(bus));
    // let mut mem_wraper = DynamicBusWrapper::new(memory);
    let mut cpu = CPU::new(&mut bus);
    cpu.program_counter = 65280; //0x8000 as u16 + pc as u16;
    // cpu.program_counter = 0xC000; //0x8000 as u16 + pc as u16;

    let trace_rc2 = trace.clone();
    cpu.interpret_fn(0xffff, |cpu| {
        // ::std::thread::sleep(Duration::new(0, 50000));
        if *trace_rc2.borrow() {
            println!("{}", rustness::cpu::trace(cpu));
        }
    });

}
