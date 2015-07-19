
use gb_memory::*;
use gb_util::*;
use gb_memory::LCDMode::*;
static MBC0_ROM : &'static str = "samples/mbc0.gb";

#[test]
fn testReadAndWriteByte() {
    let romData = match openROM(MBC0_ROM) {
        Ok(data) => data,
        Err(err) => panic!("{}", err)
    };

    assert!(romData.len() == 0x8000); //type 0 carts are 32kb long

    let mut memory = MemoryMapState::new();
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

    writeByteToMemory(&mut memory,0xAA, 0x8010) ; //writing to videoRAM
    assert!(readByteFromMemory(&memory,0x8010) ==
            memory.videoRAM[0x10]);
    assert!(readByteFromMemory(&memory,0x8010) == 0xAA); //reading from videoRAM


    writeByteToMemory(&mut memory,0xAA, 0x8010) ; //writing to videoRAM
    assert!(readByteFromMemory(&memory,0x8010) ==
            memory.videoRAM[0x10]);
    assert!(readByteFromMemory(&memory,0x8010) == 0xAA); //reading from videoRAM
}
#[test]
fn testReadAndWriteWord() {
    let romData = match openROM(MBC0_ROM) {
        Ok(data) => data,
        Err(err) => panic!("{}", err)
    };
    assert!(romData.len() == 0x8000); //type 0 carts are 32kb long

    let mut memory = MemoryMapState::new();
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

    writeWordToMemory(&mut memory,0xAAFF, 0x8000); //writing to video ram
    assert!(readWordFromMemory(&memory,0x8000) == 0xAAFF); //reading from zero page ram
}

#[test]
fn testBIOSControls() {
    let mut mem = MemoryMapState::new();
    mem.inBios = true;

    //test bios 
    writeByteToMemory(&mut mem,0x1, 0xFF50) ; //writing resets current scan line count
    //should be out of bios now
    assert_eq!(mem.inBios, false);

    assert_eq!(readByteFromMemory(&mem,0xFF50), 1);

}


#[test]
fn testLCDScanLine() {
    let mut mem = MemoryMapState::new();

    //test scanline 
    mem.currScanLine = 133;
    assert_eq!(readByteFromMemory(&mem,0xFF44), mem.currScanLine);
    writeByteToMemory(&mut mem,0xAA, 0xFF44) ; //writing resets current scan line count
    assert_eq!(readByteFromMemory(&mem,0xFF44), 0);

}

#[test]
fn testLCDStatus() {
    let mut mem = MemoryMapState::new();

    mem.lcdMode = VBlank;
    assert_eq!(readByteFromMemory(&mem,0xFF41), 1); //should be VBlank

    //TODO: Test writing to status register

}

#[test]
fn testLCDScrollReg() {
    let mut mem = MemoryMapState::new();

    writeByteToMemory(&mut mem, 32,0xFF42); //SCY
    writeByteToMemory(&mut mem, 16,0xFF43); //SCX

    assert_eq!(mem.lcdSCY, 32);
    assert_eq!(mem.lcdSCX, 16);
    assert_eq!(readByteFromMemory(&mem,0xFF42), 32); //SCY
    assert_eq!(readByteFromMemory(&mem,0xFF43), 16); //SCX


}

#[test]
fn testPalette() {
    let mut mem = MemoryMapState::new();


    writeByteToMemory(&mut mem, 0xE7, 0xFF47); 

    assert_eq!(mem.palette, [BLACK, LIGHT_GRAY, DARK_GRAY, BLACK]);

    assert_eq!(readByteFromMemory(&mem,0xFF47), 0xE7); 


}


#[test]
fn testLCDControlRegister() {
    let mut mem = MemoryMapState::new();

    assert_eq!(mem.backgroundTileMap, 0);
    assert_eq!(mem.backgroundTileSet, 0);
    assert_eq!(mem.isBackgroundEnabled, false);

    writeByteToMemory(&mut mem, 0x19, 0xFF40);

    assert_eq!(mem.backgroundTileMap, 1);
    assert_eq!(mem.backgroundTileSet, 1);
    assert_eq!(mem.isBackgroundEnabled, true);

    assert_eq!(readByteFromMemory(&mem,0xFF40), 0x19); 
}


