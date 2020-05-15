use rustness::cpu::CPU;
use rustness::screen::screen::Screen;
use std::time::Duration;

use rand::Rng;
use std::io::Write;

use crossterm::event::KeyCode;
use crossterm::event::{poll, read, Event};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, style::Color};

fn main() {
    let mut cpu = CPU::new();
    // https://gist.github.com/wkjagt/9043907
    let snake = "20 06 06 20 38 06 20 0d 06 20 2a 06 60 a9 02 85 02 a9 04 85 03 a9 11 85 10 a9 10 85 12 a9 0f 85 14 a9 04 85 11 85 13 85 15 60 a5 fe 85 00 a5 fe 29 03 18 69 02 85 01 60 20 4d 06 20 8d 06 20 c3 06 20 19 07 20 20 07 20 2d 07 4c 38 06 a5 ff c9 77 f0 0d c9 64 f0 14 c9 73 f0 1b c9 61 f0 22 60 a9 04 24 02 d0 26 a9 01 85 02 60 a9 08 24 02 d0 1b a9 02 85 02 60 a9 01 24 02 d0 10 a9 04 85 02 60 a9 02 24 02 d0 05 a9 08 85 02 60 60 20 94 06 20 a8 06 60 a5 00 c5 10 d0 0d a5 01 c5 11 d0 07 e6 03 e6 03 20 2a 06 60 a2 02 b5 10 c5 10 d0 06 b5 11 c5 11 f0 09 e8 e8 e4 03 f0 06 4c aa 06 4c 35 07 60 a6 03 ca 8a b5 10 95 12 ca 10 f9 a5 02 4a b0 09 4a b0 19 4a b0 1f 4a b0 2f a5 10 38 e9 20 85 10 90 01 60 c6 11 a9 01 c5 11 f0 28 60 e6 10 a9 1f 24 10 f0 1f 60 a5 10 18 69 20 85 10 b0 01 60 e6 11 a9 06 c5 11 f0 0c 60 c6 10 a5 10 29 1f c9 1f f0 01 60 4c 35 07 a0 00 a5 fe 91 00 60 a6 03 a9 00 81 10 a2 00 a9 01 81 10 60 60";
    let snake_u8 = CPU::transform(snake);
    cpu.program_counter = 0x600;
    cpu.memory.space[0x600..(snake_u8.len() + 0x600)].copy_from_slice(&snake_u8);

    let screen = Screen::new();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    execute!(handle, EnterAlternateScreen).unwrap();

    crossterm::terminal::enable_raw_mode().unwrap();
    execute!(handle, crossterm::cursor::Hide).unwrap();

    screen.clear(&mut handle);

    //    // game_loop(&screen, &mut handle);

    nes_loop(
        // &snake_u8,
        &cpu.memory.space.clone(),
        &mut cpu,
        &screen,
        &mut handle,
    );

    loop {
        if let Ok(true) = poll(Duration::from_millis(1)) {
            match read().unwrap() {
                Event::Key(event) => {
                    if event.code == KeyCode::Char('x') {
                        break;
                    }
                }
                _ =>
                    /* do nothing */
                    {}
            }
        }
    }

    execute!(handle, crossterm::cursor::Show).unwrap();

    crossterm::terminal::disable_raw_mode().unwrap();

    execute!(handle, LeaveAlternateScreen).unwrap();
}

fn nes_loop(program: &[u8], entry: &mut CPU, screen: &Screen, handle: &mut impl Write) {
    let mut rng = rand::thread_rng();
    let mut buff = vec![0; 1024];
    entry.interpret_fn(program, |cpu| {
        for x in 0..(4 * 32 * 8) {
            let mem = 0x0200 + (x as u16) as usize;
            let y = (x as u16) / 32;
            if cpu.memory.space[mem] != 0 || buff[x] != 0 {
                screen.draw(
                    handle,
                    (x % 32) as u16,
                    y,
                    Color::AnsiValue(cpu.memory.space[mem]),
                );
            }
            // print!("{}", cpu.memory.space[0x2]);
        }
        buff.copy_from_slice(&cpu.memory.space[0x0200..0x600]);

        if let Ok(true) = poll(Duration::from_millis(1)) {
            match read().unwrap() {
                Event::Key(event) => {
                    if event.code == KeyCode::Down {
                        cpu.memory.space[0xff] = 0x73;
                    }
                    if event.code == KeyCode::Up {
                        cpu.memory.space[0xff] = 0x77;
                    }
                    if event.code == KeyCode::Left {
                        cpu.memory.space[0xff] = 0x61;
                    }
                    if event.code == KeyCode::Right {
                        cpu.memory.space[0xff] = 0x64;
                    }

                    if event.code == KeyCode::Char('x') {
                        execute!(handle, crossterm::cursor::Show).unwrap();

                        crossterm::terminal::disable_raw_mode().unwrap();

                        execute!(handle, LeaveAlternateScreen).unwrap();
                        panic!("exit");
                    }
                }
                _ =>
                    /* do nothing */
                    {}
            }
        }

        cpu.memory.space[0xfe] = rng.gen();
    });
}
