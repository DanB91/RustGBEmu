use std::fs;
use std::io;
use std::io::Read;

pub fn hb(word: u16) -> u8 {
    (word >> 8) as u8 
}

pub fn lb(word: u16) -> u8 {
    word as u8 
}

pub fn word(high: u8, low: u8) -> u16 {
    (high as u16) << 8 | low as u16
}

pub struct MemoryState {
    pub workingRAM: [u8;0x2000],
    pub zeroPageRAM: [u8;0x7F],
    pub romData: Vec<u8>,
    pub inBios: bool
}
impl MemoryState {

    pub fn new() -> MemoryState {
        MemoryState {
            workingRAM: [0;0x2000],
            zeroPageRAM: [0;0x7F],
            romData: vec![],
            inBios: true
        }
    }


}
pub fn readByteFromMemory(memory: &MemoryState, addr: u16) -> u8 {


    let bios: [u8; 0x100] = [
        0x31, 0xFE, 0xFF, 0xAF, 0x21, 0xFF, 0x9F, 0x32, 0xCB, 0x7C, 0x20, 0xFB, 0x21, 0x26, 0xFF, 0x0E,
        0x11, 0x3E, 0x80, 0x32, 0xE2, 0x0C, 0x3E, 0xF3, 0xE2, 0x32, 0x3E, 0x77, 0x77, 0x3E, 0xFC, 0xE0,
        0x47, 0x11, 0x04, 0x01, 0x21, 0x10, 0x80, 0x1A, 0xCD, 0x95, 0x00, 0xCD, 0x96, 0x00, 0x13, 0x7B,
        0xFE, 0x34, 0x20, 0xF3, 0x11, 0xD8, 0x00, 0x06, 0x08, 0x1A, 0x13, 0x22, 0x23, 0x05, 0x20, 0xF9,
        0x3E, 0x19, 0xEA, 0x10, 0x99, 0x21, 0x2F, 0x99, 0x0E, 0x0C, 0x3D, 0x28, 0x08, 0x32, 0x0D, 0x20,
        0xF9, 0x2E, 0x0F, 0x18, 0xF3, 0x67, 0x3E, 0x64, 0x57, 0xE0, 0x42, 0x3E, 0x91, 0xE0, 0x40, 0x04,
        0x1E, 0x02, 0x0E, 0x0C, 0xF0, 0x44, 0xFE, 0x90, 0x20, 0xFA, 0x0D, 0x20, 0xF7, 0x1D, 0x20, 0xF2,
        0x0E, 0x13, 0x24, 0x7C, 0x1E, 0x83, 0xFE, 0x62, 0x28, 0x06, 0x1E, 0xC1, 0xFE, 0x64, 0x20, 0x06,
        0x7B, 0xE2, 0x0C, 0x3E, 0x87, 0xF2, 0xF0, 0x42, 0x90, 0xE0, 0x42, 0x15, 0x20, 0xD2, 0x05, 0x20,
        0x4F, 0x16, 0x20, 0x18, 0xCB, 0x4F, 0x06, 0x04, 0xC5, 0xCB, 0x11, 0x17, 0xC1, 0xCB, 0x11, 0x17,
        0x05, 0x20, 0xF5, 0x22, 0x23, 0x22, 0x23, 0xC9, 0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B,
        0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E,
        0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC,
        0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E, 0x3c, 0x42, 0xB9, 0xA5, 0xB9, 0xA5, 0x42, 0x4C,
        0x21, 0x04, 0x01, 0x11, 0xA8, 0x00, 0x1A, 0x13, 0xBE, 0x20, 0xFE, 0x23, 0x7D, 0xFE, 0x34, 0x20,
        0xF5, 0x06, 0x19, 0x78, 0x86, 0x23, 0x05, 0x20, 0xFB, 0x86, 0x20, 0xFE, 0x3E, 0x01, 0xE0, 0x50
    ];

    let i = addr as usize;
    match addr {
        0...0xFF =>  
            if memory.inBios {
                bios[i]
            }  else {
                memory.romData[i]
            },
        0x100...0x3FFF =>  
            memory.romData[i],
        0xC000...0xDFFF =>
            memory.workingRAM[i - 0xC000],
        0xE000...0xFDFF => //echo of internal RAM
            memory.workingRAM[i - 0xE000],
        0xFF80...0xFFFE =>
            memory.zeroPageRAM[i - 0xFF80],
        _ => 0
    }
}


pub fn writeByteToMemory(memory: &mut MemoryState, byte: u8, addr: u16) {
    let i = addr as usize;
    match addr {
        0xC000...0xDFFF => memory.workingRAM[i - 0xC000] = byte,
        0xE000...0xFDFF => memory.workingRAM[i - 0xE000] = byte,
        0xFF80...0xFFFE => memory.zeroPageRAM[i - 0xFF80] = byte,     
        _ => {}
    }
}

pub fn readWordFromMemory(memory: &MemoryState, addr: u16) -> u16 {
    debug_assert!(addr + 1 > addr); //check for overflow

    ((readByteFromMemory(memory, addr+1) as u16) << 8)  | 
        readByteFromMemory(memory, addr) as u16  
}

pub fn writeWordToMemory(memory: &mut MemoryState, word: u16, addr: u16 ) {
    debug_assert!(addr + 1 > addr); //check for overflow

    writeByteToMemory(memory, lb(word), addr);
    writeByteToMemory(memory, hb(word), addr+1);
}
pub fn openROM(fileName: &str) -> io::Result<Vec<u8>> {

    let mut data: Vec<u8> = vec![];
    let mut f = try!(fs::File::open(fileName)); 
    try!(f.read_to_end(&mut data));

    Ok(data)
}


#[cfg(test)]
mod tests {
    use super::*;
    static MBC0_ROM : &'static str = "samples/mbc0.gb";

#[test]
    fn testReadAndWriteByte() {
        let romData = match openROM(MBC0_ROM) {
            Ok(data) => data,
            Err(err) => panic!("{}", err)
        };

        assert!(romData.len() == 0x8000); //type 0 carts are 32kb long

        let mut memory = MemoryState::new();
        memory.romData = romData;

        assert!(readByteFromMemory(&memory,0) == 0x31); //reading from bios

        memory.inBios = false;
        assert!(readByteFromMemory(&memory,0) == 0xC3); //reading from rom

        memory.inBios = true;
        assert!(readByteFromMemory(&memory,0xC001) == 0); //reading from working ram

        writeByteToMemory(&mut memory,0xAA, 0xDFFF) ; //writing to working ram
        assert!(readByteFromMemory(&memory,0xDFFF) ==
                memory.workingRAM[memory.workingRAM.len()-1]);
        assert!(readByteFromMemory(&memory,0xDFFF) == 0xAA); //reading from working ram


        writeByteToMemory(&mut memory,0xAA, 0xE000) ; //test echo ram
        assert!(readByteFromMemory(&memory,0xC000) == 0xAA); //reading from working ram
        assert!(readByteFromMemory(&memory,0xE000) == 0xAA); //reading from working ram
       

        writeByteToMemory(&mut memory,0xAA, 0xFF90) ; //writing to zero page ram
        assert!(readByteFromMemory(&memory,0xDFFF) ==
                memory.zeroPageRAM[0x10]);
        assert!(readByteFromMemory(&memory,0xFF90) == 0xAA); //reading from zero page ram
    }

#[test]
    fn testReadAndWriteWord() {
        let romData = match openROM(MBC0_ROM) {
            Ok(data) => data,
            Err(err) => panic!("{}", err)
        };
        assert!(romData.len() == 0x8000); //type 0 carts are 32kb long

        let mut memory = MemoryState::new();
        memory.romData = romData;



        assert!(readWordFromMemory(&memory,0) == 0xFE31); //reading from bios 

        memory.inBios = false;
        assert!(readWordFromMemory(&memory,0) == 0x0CC3); //reading from rom

        memory.inBios = true;
        assert!(readWordFromMemory(&memory,0xC001) == 0); //reading from working ram

        writeWordToMemory(&mut memory,0xAAFF, 0xDFFE); //writing to working ram
        assert!(readWordFromMemory(&memory,0xDFFE) ==
                word(memory.workingRAM[memory.workingRAM.len()-1],memory.workingRAM[memory.workingRAM.len()-2]));
        assert!(readWordFromMemory(&memory,0xDFFE) == 0xAAFF); //reading from working ram
        
        writeWordToMemory(&mut memory,0xAAFF, 0xFFFD); //writing to zero page ram
        assert!(readWordFromMemory(&memory,0xFFFD) == 0xAAFF); //reading from zero page ram
    }
}

