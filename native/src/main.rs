use rustness::bus::Bus;
use rustness::cpu::cpu::CPU;
use rustness::cpu::mem::Mem;
use rustness::input;
use rustness::ppu::ppu::NesPPU;
use rustness::rom::Rom;
use rustness::screen::render;
use rustness::screen::frame::Frame;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use std::time::SystemTime;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::env;

fn main() {
    let mut key_map = HashMap::new();
    key_map.insert(Keycode::Down, input::JoypadButton::DOWN);
    key_map.insert(Keycode::Up, input::JoypadButton::UP);
    key_map.insert(Keycode::Right, input::JoypadButton::RIGHT);
    key_map.insert(Keycode::Left, input::JoypadButton::LEFT);
    key_map.insert(Keycode::Space, input::JoypadButton::SELECT);
    key_map.insert(Keycode::Return, input::JoypadButton::START);
    key_map.insert(Keycode::A, input::JoypadButton::BUTTON_A);
    key_map.insert(Keycode::S, input::JoypadButton::BUTTON_B);

    let mut file = File::open(dbg!(env::args().collect::<Vec<String>>()).get(1).unwrap()).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let rom = Rom::load(&data).unwrap();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("rust nes demo", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let joystick_system = sdl_context.joystick().unwrap();

    //ignore failure - means no joystick is attached
    let _joystick = joystick_system.open(0);
    match _joystick {
        Err(_) => println!("Keyboard is used as a controller: arrows + a + s + enter + space"),
        Ok(_) => println!("Joystick is used as a controller")
    }

    joystick_system.set_event_state(true);

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

    let frame = Frame::new();
    let func = move |z: &NesPPU, joypad: &mut input::Joypad| {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => {
                    let upd = !*trace_rc.borrow();
                    trace_rc.replace(upd);
                }

                Event::KeyDown { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, true);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, false);
                    }
                }
                Event::JoyButtonDown {
                    timestamp: _,
                    which: _,
                    button_idx,
                } => {
                    match button_idx {
                        1 => joypad.set_button_pressed_status(input::JoypadButton::BUTTON_A, true),
                        2 => joypad.set_button_pressed_status(input::JoypadButton::BUTTON_B, true),
                        9 => joypad.set_button_pressed_status(input::JoypadButton::START, true),
                        8 => joypad.set_button_pressed_status(input::JoypadButton::SELECT, true),
                        _ => panic!("shouldn't happen"),
                    }
                }
                Event::JoyButtonUp {
                    timestamp: _,
                    which: _,
                    button_idx,
                } => match button_idx {
                    1 => joypad.set_button_pressed_status(input::JoypadButton::BUTTON_A, false),
                    2 => joypad.set_button_pressed_status(input::JoypadButton::BUTTON_B, false),
                    9 => joypad.set_button_pressed_status(input::JoypadButton::START, false),
                    8 => joypad.set_button_pressed_status(input::JoypadButton::SELECT, false),
                    _ => panic!("shouldn't happen"),
                },
                Event::JoyAxisMotion {
                    timestamp: _,
                        which: _,
                    axis_idx,
                    value,
                } => {
                    match (axis_idx, value) {
                        (3, -32768) => {
                            joypad.set_button_pressed_status(input::JoypadButton::LEFT, true)
                        }
                        (3, 32767) => {
                            joypad.set_button_pressed_status(input::JoypadButton::RIGHT, true)
                        }
                        (4, -32768) => {
                            joypad.set_button_pressed_status(input::JoypadButton::UP, true)
                        }
                        (4, 32767) => {
                            joypad.set_button_pressed_status(input::JoypadButton::DOWN, true)
                        }
                        (3, -129) => {
                            joypad.set_button_pressed_status(input::JoypadButton::LEFT, false);
                            joypad.set_button_pressed_status(input::JoypadButton::RIGHT, false);
                        }
                        (4, -129) => {
                            joypad.set_button_pressed_status(input::JoypadButton::UP, false);
                            joypad.set_button_pressed_status(input::JoypadButton::DOWN, false);
                        }
                        _ => { /* do nothing*/ }
                    }
                }
                _ => {}
            }
        }

        // render::render(z, &mut frame);
        texture.update(None, &z.frame.borrow().data, 256 * 3).unwrap();
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

    let pc = Mem::read_u16(&mut bus, 0xfffc);
    println!("ROM Start address: {}", pc);
    let mut cpu = CPU::new(Box::from(bus));
    cpu.program_counter = pc;

    let trace_rc2 = trace.clone();
    cpu.interpret_fn(0xffff, |cpu| {
        if *trace_rc2.borrow() {
            // ::std::thread::sleep(Duration::new(0, 10000));
            println!("{}", rustness::cpu::trace(cpu));
        }
    });
}
