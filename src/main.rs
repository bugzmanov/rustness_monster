use rustness::cpu::cpu::CPU;
use rustness::rom::ines::Rom;
use rustness::cpu::cpu::Memory;

use std::fs::File;
use std::io::Read;

use rustness::cpu::cpu::DynamicMemWrapper;
use std::cell::RefCell;
use std::rc::Rc;
use rustness::screen::frame::Frame;

fn main() {
    let memory = Rc::from(RefCell::from(Memory::new()));
    let mut mem_wraper = DynamicMemWrapper::new(memory.clone());

    let mut cpu = CPU::new(&mut mem_wraper);

    let mut file = File::open("test_rom/ice_climber.nes").unwrap();
    // let mut file = File::open("test_rom/official.nes").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let rom = Rom::load(&data).unwrap();
    cpu.program_counter = 0x6000;
    println!("{}", rom.prg_rom.len());

    let tile = &rom.chr_rom[0..15];

    let buf = vec![0; 64];
    let mut frame = Frame::new();
    for y in 0..7 {
        let mut upper = tile[y];
        let mut lower = tile[y + 8];
        
        for x in 0..7 {
            let value = 1 & upper << 1 | 1 & lower;
            upper = upper >> 1;
            lower = lower >> 1;
            let rgb = match value {
                0 => (0,0,0),
                1 => (255,0,0),
                2 => (0, 255,0),
                3 => (0,0,255),
                _ => panic!("can't be"),
            };

            frame.set_pixel(x, y, rgb)
        }
    }

    // cpu.memory.space[0..rom.prg_rom.len()].copy_from_slice(&rom.prg_rom);
    //

    // cpu.interpret(&cpu.memory.space.clone());
}
