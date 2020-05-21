use rustness::cpu::cpu::CPU;
use rustness::nes::ines::Rom;

use std::fs::File;
use std::io::Read;

fn main() {
    let mut cpu = CPU::new();

    let mut file = File::open("test_rom/cpu_dummy_reads.nes").unwrap();
    // let mut file = File::open("test_rom/official.nes").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let rom = Rom::load(&data).unwrap();
    cpu.program_counter = 0x6000;
    println!("{}", rom.prg_rom.len());

    cpu.memory.space[0..rom.prg_rom.len()].copy_from_slice(&rom.prg_rom);
    // let ololo = &cpu.memory()space[0x6000..(0x6000 + 1024)];
    //

    cpu.interpret(&cpu.memory.space.clone());
    // let ololo = &cpu.memory()space[0x6000..(0x6000 + 1024)];
}
