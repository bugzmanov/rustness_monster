use rustness::bus::bus::Bus;
use rustness::cpu::cpu::CPU;
use rustness::cpu::mem::Mem;
use rustness::input;
use rustness::ppu::ppu::NesPPU;
use rustness::rom::ines::Rom;
use std::io::Read;

use rustness::bus::bus::DynamicBusWrapper;
use std::cell::RefCell;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::rc::Rc;
fn main() {
    // let mut file = File::open("test_rom/ice_climber.nes").unwrap();
    let mut file = File::open("test_rom/nestest.nes").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let rom = Rom::load(&data).unwrap();

    let func = |_: &NesPPU, _: &mut input::Joypad| {
        // do nothing
    };

    let mut bus = Bus::<NesPPU>::new(rom, func);

    let start_pc = Mem::read_u16(&mut bus, 0xfffc);

    let memory = Rc::from(RefCell::from(bus));
    let mem_wraper = DynamicBusWrapper::new(memory.clone());
    let mut cpu = CPU::new(Box::from(mem_wraper));
    cpu.program_counter = start_pc; //0x8000 as u16 + pc as u16;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        // .append(true)
        .open("nestest.log")
        .unwrap();

    cpu.interpret_fn(0xffff, |cpu| {
        file.write_all(&(rustness::cpu::trace(cpu) + "\n").as_bytes())
            .unwrap();
        file.flush().unwrap();
        println!("{}", rustness::cpu::trace(cpu));
    });
}
