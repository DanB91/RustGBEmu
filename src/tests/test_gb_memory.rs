
use gb_memory::*;
use gb_util::*;
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
            memory.lcd.videoRAM[0x10]);
    assert!(readByteFromMemory(&memory,0x8010) == 0xAA); //reading from videoRAM


    writeByteToMemory(&mut memory,0xAA, 0x8010) ; //writing to videoRAM
    assert!(readByteFromMemory(&memory,0x8010) ==
            memory.lcd.videoRAM[0x10]);
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




