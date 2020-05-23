use rustness::cpu::cpu::CPU;
use rustness::rom::ines::Rom;
use rustness::cpu::cpu::Memory;

use std::fs::File;
use std::io::Read;

use rustness::cpu::cpu::DynamicMemWrapper;
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let memory = Rc::from(RefCell::from(Memory::new()));
    let mut mem_wraper = DynamicMemWrapper::new(memory.clone());

    let mut cpu = CPU::new(&mut mem_wraper);

    let mut file = File::open("test_rom/cpu_dummy_reads.nes").unwrap();
    // let mut file = File::open("test_rom/official.nes").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let rom = Rom::load(&data).unwrap();
    cpu.program_counter = 0x6000;
    println!("{}", rom.prg_rom.len());

    // cpu.memory.space[0..rom.prg_rom.len()].copy_from_slice(&rom.prg_rom);
    //

    // cpu.interpret(&cpu.memory.space.clone());
}
