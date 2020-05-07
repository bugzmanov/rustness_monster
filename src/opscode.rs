use crate::cpu::AddressingMode;
use std::collections::HashMap;


pub struct OpsCode {
    pub code: u8,
    pub mnemonic: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl OpsCode {

    fn new(code: u8, mnemonic: &'static str, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        OpsCode {
            code: code,
            mnemonic: mnemonic,
            len: len,
            cycles: cycles,
            mode: mode,   
        }
    }
 }

 lazy_static! {
    pub static ref CPU_OPS_CODES: Vec<OpsCode> = vec![
        OpsCode::new(0xa9, "LDA", 2, 2, AddressingMode::Immediate), 
        OpsCode::new(0xa5, "LDA", 2, 3, AddressingMode::ZeroPage), 
        OpsCode::new(0xb5, "LDA", 2, 4, AddressingMode::ZeroPage_X), 
        OpsCode::new(0xad, "LDA", 3, 4, AddressingMode::Absolute), 
        OpsCode::new(0xbd, "LDA", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_X), 
        OpsCode::new(0xb9, "LDA", 3, 4/*+1 if page crossed*/, AddressingMode::Absolute_Y), 
        OpsCode::new(0xa1, "LDA", 2, 6/*+1 if page crossed*/, AddressingMode::Indirect_X), 
        OpsCode::new(0xb1, "LDA", 2, 5/*+1 if page crossed*/, AddressingMode::Indirect_Y), 

        OpsCode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage), 
        OpsCode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPage_X), 
        OpsCode::new(0x8d, "STA", 3, 4, AddressingMode::Absolute), 
        OpsCode::new(0x9d, "STA", 3, 5, AddressingMode::Absolute_X), 
        OpsCode::new(0x99, "STA", 3, 5, AddressingMode::Absolute_Y), 
        OpsCode::new(0x81, "STA", 2, 6, AddressingMode::Indirect_X), 
        OpsCode::new(0x91, "STA", 2, 6, AddressingMode::Indirect_Y), 
        
        OpsCode::new(0x18, "CLC", 1, 2, AddressingMode::None_Addressing), 


    ];

    pub static ref OPSCODES_MAP: HashMap<u8, &'static OpsCode> = {
        let mut map = HashMap::new();
        for cpuop in &*CPU_OPS_CODES {
            map.insert(cpuop.code, cpuop);
        }
        // map.insert(0xa9, &CPU_OPS_CODES[0]);
        // map.insert(0xa5, &CPU_OPS_CODES[1]);
        // map.insert(0xb5, &CPU_OPS_CODES[2]);
        // map.insert(0xad, &CPU_OPS_CODES[3]);
        // map.insert(0xbd, &CPU_OPS_CODES[4]);
        // map.insert(0xb9, &CPU_OPS_CODES[4]);
        map
    };
 }
