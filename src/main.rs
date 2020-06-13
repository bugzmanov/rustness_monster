use rustness::bus::bus::Bus;
use rustness::cpu::cpu::CPU;
use rustness::input;
use rustness::ppu::ppu;
use rustness::ppu::ppu::NesPPU;
use rustness::rom::ines::Rom;
use std::io::Read;

use rustness::bus::bus::DynamicBusWrapper;
use std::cell::RefCell;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::{rc::Rc, time::Duration};
fn main() {
    // let mut file = File::open("test_rom/ice_climber.nes").unwrap();
    let mut file = File::open("test_rom/nestest.nes").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let rom = Rom::load(&data).unwrap();

    let func = |z: &NesPPU, _: &mut input::Joypad| {
        // let frame = ppu::render(z);
    };

    let mut bus = Bus::<NesPPU>::new(rom, func);

    let pc = bus.read(0xfffc);
    let ffd = bus.read(0xfffd);

    let memory = Rc::from(RefCell::from(bus));
    let mem_wraper = DynamicBusWrapper::new(memory.clone());
    let mut cpu = CPU::new(Box::from(mem_wraper));
    cpu.program_counter = 0xc000; //0x8000 as u16 + pc as u16;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        // .append(true)
        .open("nestest.log")
        .unwrap();

    cpu.interpret_fn(0xffff, |cpu| {
        // ::std::thread::sleep(Duration::new(0, 50000));

        file.write_all(&(rustness::cpu::trace(cpu) + "\n").as_bytes())
            .unwrap();
        // buffer.write_lin &rustness::cpu::trace(cpu)).unwrap();
        file.flush().unwrap();
        println!("{}", rustness::cpu::trace(cpu));
    });
}
