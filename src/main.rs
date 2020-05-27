use rustness::bus::bus::Bus;
use rustness::cpu::cpu::CPU;
use rustness::rom::ines::Rom;

use std::fs::File;
use std::io::Read;

use rustness::bus::bus::DynamicBusWrapper;
use std::cell::RefCell;
use std::{rc::Rc, time::Duration};

fn main() {
    // let mut file = File::open("test_rom/ice_climber.nes").unwrap();
    let mut file = File::open("test_rom/pacman.nes").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let rom = Rom::load(&data).unwrap();

    let bus = Bus::new(rom);

    let pc = bus.read(0xfffc);
    let ffd = bus.read(0xfffd);

    let memory = Rc::from(RefCell::from(bus));
    let mut mem_wraper = DynamicBusWrapper::new(memory.clone());
    let mut cpu = CPU::new(&mut mem_wraper);
    cpu.program_counter = 65280; //0x8000 as u16 + pc as u16;

    cpu.interpret_fn(0xffff, |cpu| {
        ::std::thread::sleep(Duration::new(0, 50000000));
        println!("{}", rustness::cpu::trace(cpu));
    });
}
