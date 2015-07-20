
use gb_memory::*;
use gb_lcd::*;
use gb_lcd::LCDMode::*;
static MBC0_ROM : &'static str = "samples/mbc0.gb";

#[test]
fn testLCDScanLine() {
    let mut mem = MemoryMapState::new();

    //test scanline 
    mem.lcd.currScanLine = 133;
    assert_eq!(readByteFromMemory(&mem,0xFF44), mem.lcd.currScanLine);
    writeByteToMemory(&mut mem,0xAA, 0xFF44) ; //writing resets current scan line count
    assert_eq!(readByteFromMemory(&mem,0xFF44), 0);

}

#[test]
fn testLCDStatus() {
    let mut mem = MemoryMapState::new();

    mem.lcd.mode = VBlank;
    assert_eq!(readByteFromMemory(&mem,0xFF41), 1); //should be VBlank

    //TODO: Test writing to status register

}

#[test]
fn testLCDScrollReg() {
    let mut mem = MemoryMapState::new();

    writeByteToMemory(&mut mem, 32,0xFF42); //SCY
    writeByteToMemory(&mut mem, 16,0xFF43); //SCX

    assert_eq!(mem.lcd.scy, 32);
    assert_eq!(mem.lcd.scx, 16);
    assert_eq!(readByteFromMemory(&mem,0xFF42), 32); //SCY
    assert_eq!(readByteFromMemory(&mem,0xFF43), 16); //SCX


}

#[test]
fn testPalette() {
    let mut mem = MemoryMapState::new();


    writeByteToMemory(&mut mem, 0xE7, 0xFF47); 

    assert_eq!(mem.lcd.palette, [BLACK, LIGHT_GRAY, DARK_GRAY, BLACK]);

    assert_eq!(readByteFromMemory(&mem,0xFF47), 0xE7); 


}


#[test]
fn testLCDControlRegister() {
    let mut mem = MemoryMapState::new();

    assert_eq!(mem.lcd.backgroundTileMap, 0);
    assert_eq!(mem.lcd.backgroundTileSet, 0);
    assert_eq!(mem.lcd.isBackgroundEnabled, false);

    writeByteToMemory(&mut mem, 0x19, 0xFF40);

    assert_eq!(mem.lcd.backgroundTileMap, 1);
    assert_eq!(mem.lcd.backgroundTileSet, 1);
    assert_eq!(mem.lcd.isBackgroundEnabled, true);

    assert_eq!(readByteFromMemory(&mem,0xFF40), 0x19); 
}
