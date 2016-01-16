use std::fs;
use std::io;
use std::io::Read;

use gb_util::*;
use gb_lcd::*;
use gb_joypad::*;
use gb_cpu::CLOCK_SPEED_HZ;

pub const CYCLES_PER_DIVIDER_INCREMENT: u32 = 256;
pub const CYCLES_PER_DMA_BYTE: u32 = 4;


//tells how fast to increment the timer
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum TimerMode  {
    Mode0 =  CLOCK_SPEED_HZ as u32 / 4096,
    Mode1 =  CLOCK_SPEED_HZ as u32 / 262144,
    Mode2 =  CLOCK_SPEED_HZ as u32 / 65536,
    Mode3 =  CLOCK_SPEED_HZ as u32 / 16384
}

#[derive(PartialEq)]
#[repr(u8)]
pub enum MemoryBankControllerType {
    MBC0 = 0,
    MBC1 = 1,
}

#[repr(u8)]
pub enum BankingMode {
    Mode0 = 0, //8kbyte ram; 2MB rom
    Mode1 = 1  //32kbyte ram; 512kb ROM
}


//NOTE(DanB):anything accessed by MMU goes in here including LCD related function
pub struct MemoryMapState {
    pub workingRAM: [u8;0x2000],
    pub zeroPageRAM: [u8;0x7F],
    pub inBios: bool,

    pub requestedInterrupts: u8,
    pub enabledInterrupts: u8,

    pub lcd: LCDState,
    pub joypad: JoypadState,

    //timer registers
    pub divider: u8, //DIV
    pub timerCounter: u8, //TIMA
    pub timerModulo: u8, //TMA
    pub timerMode: TimerMode, 
    pub isTimerEnabled: bool, 

    pub isDMAOccurring: bool,
    pub currentDMAAddress: u16,
    pub currentDMACycles: u32,


    //Cart data
    pub romData: Vec<u8>,
    pub mbcType: MemoryBankControllerType,
    pub cartRAM: Vec<u8>,

    pub currentMBCBank: u8,
    pub currentRAMBank: u8,
    pub isCartRAMEnabled: bool,
    pub bankingMode: BankingMode
}

impl MemoryMapState {

    pub fn new() -> MemoryMapState {
        MemoryMapState {
            workingRAM: [0;0x2000],
            zeroPageRAM: [0;0x7F],
            inBios: true,

            requestedInterrupts: 0,
            enabledInterrupts: 0,

            lcd: LCDState::new(),
            joypad: JoypadState::new(),

            divider: 0,
            timerCounter: 0,
            timerModulo: 0,
            timerMode: TimerMode::Mode0, 
            isTimerEnabled: false,

            isDMAOccurring: false,
            currentDMAAddress: 0,
            currentDMACycles: 0,

            romData: vec![],
            mbcType: MemoryBankControllerType::MBC0,
            cartRAM: vec![],
            currentMBCBank: 0,
            currentRAMBank: 0,
            isCartRAMEnabled: false,
            bankingMode: BankingMode::Mode0
        }
    }


}

static BIOS: [u8; 0x100] = [
    0x31, 0xFE, 0xFF, 0xAF, 0x21, 0xFF, 0x9F, 0x32,
    0xCB, 0x7C, 0x20, 0xFB, 0x21, 0x26, 0xFF, 0x0E,
    0x11, 0x3E, 0x80, 0x32, 0xE2, 0x0C, 0x3E, 0xF3,
    0xE2, 0x32, 0x3E, 0x77, 0x77, 0x3E, 0xFC, 0xE0,
    0x47, 0x11, 0x04, 0x01, 0x21, 0x10, 0x80, 0x1A,
    0xCD, 0x95, 0x00, 0xCD, 0x96, 0x00, 0x13, 0x7B,
    0xFE, 0x34, 0x20, 0xF3, 0x11, 0xD8, 0x00, 0x06,
    0x08, 0x1A, 0x13, 0x22, 0x23, 0x05, 0x20, 0xF9,
    0x3E, 0x19, 0xEA, 0x10, 0x99, 0x21, 0x2F, 0x99,
    0x0E, 0x0C, 0x3D, 0x28, 0x08, 0x32, 0x0D, 0x20,
    0xF9, 0x2E, 0x0F, 0x18, 0xF3, 0x67, 0x3E, 0x64,
    0x57, 0xE0, 0x42, 0x3E, 0x91, 0xE0, 0x40, 0x04,
    0x1E, 0x02, 0x0E, 0x0C, 0xF0, 0x44, 0xFE, 0x90,
    0x20, 0xFA, 0x0D, 0x20, 0xF7, 0x1D, 0x20, 0xF2,
    0x0E, 0x13, 0x24, 0x7C, 0x1E, 0x83, 0xFE, 0x62,
    0x28, 0x06, 0x1E, 0xC1, 0xFE, 0x64, 0x20, 0x06,
    0x7B, 0xE2, 0x0C, 0x3E, 0x87, 0xE2, 0xF0, 0x42,
    0x90, 0xE0, 0x42, 0x15, 0x20, 0xD2, 0x05, 0x20,
    0x4F, 0x16, 0x20, 0x18, 0xCB, 0x4F, 0x06, 0x04,
    0xC5, 0xCB, 0x11, 0x17, 0xC1, 0xCB, 0x11, 0x17,
    0x05, 0x20, 0xF5, 0x22, 0x23, 0x22, 0x23, 0xC9,
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B,
    0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E,
    0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC,
    0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
    0x3C, 0x42, 0xB9, 0xA5, 0xB9, 0xA5, 0x42, 0x3C,
    0x21, 0x04, 0x01, 0x11, 0xA8, 0x00, 0x1A, 0x13,
    0xBE, 0x20, 0xFE, 0x23, 0x7D, 0xFE, 0x34, 0x20,
    0xF5, 0x06, 0x19, 0x78, 0x86, 0x23, 0x05, 0x20,
    0xFB, 0x86, 0x20, 0xFE, 0x3E, 0x01, 0xE0, 0x50
];

fn u8ForColorPalette(colorPalette: &[PaletteColor]) -> u8 {
    let mut colorReg = 0;

    for i in 0..colorPalette.len() {
        let color = match colorPalette[i] {
            WHITE => 0,
            LIGHT_GRAY => 1,
            DARK_GRAY => 2,
            BLACK => 3,
            _ => panic!("Only 4 colors implmented so far...")
        };
        colorReg |= color << (i * 2);

    }

    colorReg
    
}

fn updateColorPaletteFromU8(colorPalette: &mut [PaletteColor], val: u8) {
    for i in 0..colorPalette.len() {
        let colorNum = (val >> 2 * i) & 3; 

        colorPalette[i] = match colorNum {
            0 => WHITE,
            1 => LIGHT_GRAY,
            2 => DARK_GRAY,
            3 => BLACK,
            _ => panic!("Only 4 colors available.  Bad color: {}", colorNum)
        };
    }

}

pub fn readByteFromMemory(memory: &MemoryMapState, addr: u16) -> u8 {
    use gb_lcd::LCDMode::*;

    let lcd = &memory.lcd;
    let joypad = &memory.joypad;

    let i = addr as usize;
    match addr {
        0...0xFF if memory.inBios => BIOS[i],  
        0...0xFF if !memory.inBios => memory.romData[i], 
        0x100...0x3FFF => memory.romData[i],
        0x4000...0x7FFF =>{
            match memory.mbcType {
                MemoryBankControllerType::MBC0 => memory.romData[i],
                MemoryBankControllerType::MBC1 => {
                    let addrMultiplier = memory.currentMBCBank as usize + 1;
                    memory.cartRAM[i * addrMultiplier]

                }
            }
        }
        0x8000...0x9FFF => {
            //vram can only be properly accessed when not being drawn from
            if lcd.mode != ScanVRAMAndOAM {
                lcd.videoRAM[i - 0x8000]
            }
            else {
                0xFF
            }
        }
        0xA000...0xBFFF => {

            if memory.isCartRAMEnabled {
                match memory.mbcType {
                    MemoryBankControllerType::MBC0 => memory.cartRAM[i - 0xA000],
                    MemoryBankControllerType::MBC1 => {
                       let addrMultiplier = memory.currentRAMBank as usize + 1;
                       memory.cartRAM[(i - 0xA000) * addrMultiplier]
                    },
                }
            }
            else {
                0
            }

        }
        0xC000...0xDFFF => memory.workingRAM[i - 0xC000],
        0xE000...0xFDFF => memory.workingRAM[i - 0xE000], //echo of internal RAM 
        0xFE00...0xFE9F => {
            if lcd.mode != ScanVRAMAndOAM && lcd.mode != ScanOAM {
                lcd.oam[i - 0xFE00]
            }
            else {
                0xFF
            }
        },
        0xFF00 => { //Joypad register
            let mut joypReg = 0u8;

            match joypad.selectedButtonGroup {
                ButtonGroup::DPad => {
                    joypReg = 0x20;

                    joypReg |= (joypad.down as u8) << 3;
                    joypReg |= (joypad.up as u8) << 2;
                    joypReg |= (joypad.left as u8) << 1;
                    joypReg |= joypad.right as u8;
                }

                ButtonGroup::FaceButtons => {
                    joypReg = 0x10;

                    joypReg |= (joypad.start as u8) << 3;
                    joypReg |= (joypad.select as u8) << 2;
                    joypReg |= (joypad.b as u8) << 1;
                    joypReg |= joypad.a as u8;
                }

                ButtonGroup::Nothing => {}
            }

            joypReg
        },
        0xFF04 => memory.divider,
        0xFF05 => memory.timerCounter,
        0xFF06 => memory.timerModulo,
        0xFF07 => { //TAC
            let mut tacReg = 
                match memory.timerMode {
                    TimerMode::Mode0 => 0,
                    TimerMode::Mode1 => 1,
                    TimerMode::Mode2 => 2,
                    TimerMode::Mode3 => 3
                };

            if memory.isTimerEnabled {
                tacReg |= 1 << 2;
            }
            
            tacReg

        },

        0xFF0F => memory.requestedInterrupts,
        0xFF40 => { //LCD Control
            let mut control = 0u8;

            //Bit 7 - LCD Enabled
            control = if lcd.isEnabled { control | (1 << 7)} else {control};
            //TODO: Tile stuff goes here

            //Bit 4 - Background Tile Set Select
            control |= lcd.backgroundTileSet << 4;
            //Bit 3 - Background Tile Data Select
            control |= lcd.backgroundTileMap << 3;
            //Bit 2 - Sprite size
            control |= match lcd.spriteHeight {
                SpriteHeight::Short => 0, //bit unset in 8x8 mode
                SpriteHeight::Tall => (1 << 2), //bit set in 8x16 mode
            };

            //Bit 1 - OAM Enable
            control |= if lcd.isOAMEnabled {1 << 1} else {0};

            //Bit 0 - Background enabled
            control |= if lcd.isBackgroundEnabled {1} else {0};

            control
        },


        0xFF41 => { //LCD Status
            //TODO: move orring of lcd.mode bits into gb_lcd.rs
            let mut status = lcd.lcdc;
            status |= lcd.mode as u8; //put in lcd mode

            status
        },

        0xFF42 => lcd.scy,
        0xFF43 => lcd.scx,
        0xFF44 => lcd.currScanLine,
        0xFF45 => lcd.lyc,
        0xFF46 => (memory.currentDMAAddress >> 8) as u8,
        0xFF47 => u8ForColorPalette(&lcd.palette),
        0xFF48 => u8ForColorPalette(&lcd.spritePalette0),
        0xFF49 => u8ForColorPalette(&lcd.spritePalette1),
        0xFF50 => if memory.inBios {0} else {1},
        0xFF80...0xFFFE => memory.zeroPageRAM[i - 0xFF80],
        0xFFFF => memory.enabledInterrupts,
        _ => 0
    }
}


pub fn writeByteToMemory(memory: &mut MemoryMapState, byte: u8, addr: u16) {
    use gb_lcd::LCDMode::*;

    let lcd = &mut memory.lcd;
    let joypad = &mut memory.joypad;

    let i = addr as usize;
    match addr {
        0...0x1FFF => {
            if byte == 0xA {
                memory.isCartRAMEnabled = true;
            }
            else {
                memory.isCartRAMEnabled = false;
            }
        }
        0x2000...0x3FFF => {
            match memory.bankingMode {
                BankingMode::Mode0 => {}
                BankingMode::Mode1 => memory.currentRAMBank |= byte & 0x1F,
            }
            match memory.currentMBCBank {
                0|0x20|0x40|0x60 => memory.currentMBCBank += 1,
                _ => {}
                
            }
        }
        0x4000...0x5FFF => {
            match memory.bankingMode {
                BankingMode::Mode0 => memory.currentMBCBank |= byte << 5,
                BankingMode::Mode1 => memory.currentRAMBank |= byte << 5,
            }
            match memory.currentMBCBank {
                0|0x20|0x40|0x60 => memory.currentMBCBank += 1,
                _ => {}
                
            }
        }
        //vram can only be properly accessed when not being drawn from
        0x8000...0x9FFF if lcd.mode != ScanVRAMAndOAM => lcd.videoRAM[i - 0x8000] = byte,
        0xA000...0xBFFF => {

            if memory.isCartRAMEnabled {
                match memory.mbcType {
                    MemoryBankControllerType::MBC0 => memory.cartRAM[i - 0xA000] = byte ,
                    MemoryBankControllerType::MBC1 => {
                       let addrMultiplier = memory.currentRAMBank as usize + 1;
                       memory.cartRAM[(i - 0xA000) * addrMultiplier] = byte
                    },
                }
            }

        }
        0xC000...0xDFFF => memory.workingRAM[i - 0xC000] = byte,
        0xE000...0xFDFF => memory.workingRAM[i - 0xE000] = byte,
        //TODO: Shouldn't be able to write to OAM memory during these modes.
        //      Enabling commented out code breaks sprites.  Figure out why...
        0xFE00...0xFE9F /*if lcd.mode != ScanVRAMAndOAM && lcd.mode != ScanOAM*/ => 
            lcd.oam[i - 0xFE00] = byte,
        0xFF00 => {//Joypad Register
            joypad.selectedButtonGroup =
                match byte & 0x30 { //only look at bits 4 and 5
                    0x20 => ButtonGroup::DPad,
                    0x10 => ButtonGroup::FaceButtons,
                    0x30 | 0 => ButtonGroup::Nothing,
                    _ => panic!("This really would be an error in the compiler if we hit here")
                }

        },
        0xFF04 => memory.divider = 0, //reset divider
        0xFF05 => memory.timerCounter = memory.timerModulo,
        0xFF06 => memory.timerModulo = byte,
        0xFF07 =>  { //TAC
            memory.timerMode = match byte & 3 {
                0 => TimerMode::Mode0,
                1 => TimerMode::Mode1,
                2 => TimerMode::Mode2,
                3 => TimerMode::Mode3,
                _ => panic!("This is impossible...")
            };

            memory.isTimerEnabled = 
                if byte & (1 << 2) != 0 { 
                    true
                }
                else {
                    false
                };
        },
        0xFF0F => memory.requestedInterrupts = byte,
        0xFF40 => { //LCD Control

            //Bit 7 - LCD Enabled
            lcd.isEnabled = if (byte & 0x80) != 0 {true} else {false};
            
            //Bit 4 - Background Tile Set Select
            lcd.backgroundTileSet = if testBit!(byte, 4) {1} else {0};
            //Bit 3 - Background Tile Map Select
            lcd.backgroundTileMap = if testBit!(byte, 3) {1} else {0};
            //Bit 2 - Sprite size
            lcd.spriteHeight = if testBit!(byte, 2) {SpriteHeight::Tall} 
                else {SpriteHeight::Short};
            //Bit 1 - OAM enabled
            lcd.isOAMEnabled = testBit!(byte, 1);
            //Bit 0 - Background enabled
            lcd.isBackgroundEnabled = (byte & 1) != 0;

        },
        0xFF41 => {//Configure LCDC interrupt 
            //last 3 bits are read-only
            lcd.lcdc = byte & !7

        },
        0xFF42 => lcd.scy = byte,
        0xFF43 => lcd.scx = byte,
        0xFF44 => lcd.currScanLine = 0, //resets the current line if written to
        0xFF45 => lcd.lyc = byte,
        
        0xFF46 => {
            memory.currentDMAAddress = if byte <= 0xF1 {
                0x100 * byte as u16
            }
            else {
                0xF100
            };

            memory.isDMAOccurring = true;
        },

        0xFF47 => updateColorPaletteFromU8(&mut lcd.palette, byte),
        0xFF48 => updateColorPaletteFromU8(&mut lcd.spritePalette0, byte),
        0xFF49 => updateColorPaletteFromU8(&mut lcd.spritePalette1, byte),
        //TODO: Implement writing to LCD status
        0xFF50 => memory.inBios = if byte != 0 {false} else {true},
        0xFF80...0xFFFE => memory.zeroPageRAM[i - 0xFF80] = byte, 
        0xFFFF => memory.enabledInterrupts = byte,
        _ => {}
    }
}

pub fn readWordFromMemory(memory: &MemoryMapState, addr: u16) -> u16 {
    debug_assert!(addr.wrapping_add(1) > addr); //check for overflow

    ((readByteFromMemory(memory, addr+1) as u16) << 8)  | 
        readByteFromMemory(memory, addr) as u16  
}

pub fn writeWordToMemory(memory: &mut MemoryMapState, word: u16, addr: u16 ) {
    debug_assert!(addr.wrapping_add(1) > addr, 
                  "Word {:X} being inserted at address {:X}", word, addr); //check for overflow

    writeByteToMemory(memory, lb(word), addr);
    writeByteToMemory(memory, hb(word), addr+1);
}
pub fn openROM(fileName: &str) -> io::Result<Vec<u8>> {

    let mut data: Vec<u8> = vec![];
    let mut f = try!(fs::File::open(fileName)); 
    try!(f.read_to_end(&mut data));

    Ok(data)
}




