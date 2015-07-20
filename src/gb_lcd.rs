extern crate sdl2;
use std::mem::swap; 

//Holds the state of the  screen and controller
pub struct LCDState {
    pub palette: [Color;4], //color palette
    pub videoRAM: [u8;0x2000],
    pub mode: LCDMode, 
    pub modeClock: u32,
    pub scx: u8, //scroll x
    pub scy: u8, //scroll y
    pub currScanLine: u8,
    pub backgroundTileMap: u8, //which background tile map to use (0 or 1)
    pub backgroundTileSet: u8, //which background tile set to use (0 or 1)
    pub isBackgroundEnabled: bool,
    pub isEnabled: bool,
    
    pub screen: LCDScreen,
    pub screenBackBuffer: LCDScreen,
}

#[derive(PartialEq, Copy, Clone)]
pub enum LCDMode {
    HBlank = 0,
    VBlank = 1, 
    ScanOAM = 2,
    ScanVRAM = 3
}

pub type Color = sdl2::pixels::Color;
pub const WHITE: Color = sdl2::pixels::Color::RGBA(255, 255, 255, 255);
pub const LIGHT_GRAY: Color = sdl2::pixels::Color::RGBA(170,170,170,255);
pub const DARK_GRAY: Color = sdl2::pixels::Color::RGBA(85,85,85,255);
pub const BLACK: Color = sdl2::pixels::Color::RGBA(0,0,0,255);

pub type LCDScreen = [[Color;160];144]; 
pub const BLANK_SCREEN: LCDScreen = [[WHITE;160];144];

impl LCDState {

    pub fn new() -> LCDState {
        LCDState {
            
            mode: LCDMode::ScanOAM,
            modeClock: 0,
            scx: 0, //scroll x
            scy: 0, //scroll y
            currScanLine: 0,
            videoRAM: [0;0x2000],
            backgroundTileMap: 0, //which tile map to use (0 or 1)
            backgroundTileSet: 0, //which tile set to use (0 or 1)
            isBackgroundEnabled: false,
            palette: [WHITE, WHITE, WHITE, WHITE], //color pallet
            isEnabled: false,

            screen: BLANK_SCREEN,
            screenBackBuffer: BLANK_SCREEN
        }
    }


}

pub fn stepLCD(lcd: &mut LCDState, cyclesTakenOfLastInstruction: u32) {
    use self::LCDMode::*;

    if lcd.isEnabled {
        
        //get instruction cycles of last instruction exectued
        lcd.modeClock += cyclesTakenOfLastInstruction; 
        
        match lcd.mode {

            HBlank if lcd.modeClock >= 204 => {
                lcd.modeClock = 0;
                lcd.currScanLine += 1;

                //at the last line...
                if lcd.currScanLine == 143 {
                    lcd.mode = VBlank; //engage VBlank
                    swap(&mut lcd.screen, &mut lcd.screenBackBuffer); //commit fully drawn screen

                }
                else {
                    lcd.mode = ScanOAM;
                }
            },

            VBlank if lcd.modeClock >= 456 => {
                lcd.currScanLine += 1;
                lcd.modeClock = 0;

                if lcd.currScanLine == 153 {
                    lcd.mode = ScanOAM;
                    lcd.currScanLine = 0;

                }
            },

            ScanOAM if lcd.modeClock >= 80 => {
                //TODO: Draw OAM to internal screen buffer


                lcd.mode = ScanVRAM;
                lcd.modeClock = 0;
            },

            ScanVRAM if lcd.modeClock >= 172 => {
                //TODO: Draw VRAM to internal screen buffer

                let y = lcd.scy.wrapping_add(lcd.currScanLine);

                //draw background
                if lcd.isBackgroundEnabled {
                    let mut tileRefAddr = match lcd.backgroundTileMap {
                        0 => 0x1800usize,  //it is 0x1800 instead of 0x9800 because this is relative to start of vram
                        1 => 0x1C00usize,
                        _ => panic!("Uh oh, the tile map should only be 0 or 1")
                    };

                    /* Tile Map:
                     *
                     * Each "row" is 32 bytes long where each byte is a tile reference
                     * Each byte represents a 8x8 pixel tils, so each row and column are 256 pixels long
                     * Each byte represents a 16 byte tile where every 2 bytes represents an 8 pixel row
                     *
                     *------------------------------------------------------
                     *|tile ref | tile ref | ...............................
                     *|-----------------------------------------------------
                     *|tile ref | tile ref | ...............................
                     *|.
                     *|.
                     *|.
                     */
                    tileRefAddr += (y as usize / 8) * 32; //which tile in the y dimension?

                    let tileRefRowStart = tileRefAddr; // start of the row in the 32x32 tile map

                    tileRefAddr += lcd.scx as usize / 8; //which tile in x dimension?

                    //the x pixel is gotten by shifting a mask of the form 100000
                    let mut xMask = 0x80u8 >> (lcd.scx & 7);

                    for x in 0..160 {

                        let tileRef = lcd.videoRAM[tileRefAddr];

                        //find the tile based on the tile reference
                        let mut tileAddr = match lcd.backgroundTileSet {
                            0 => (0x1000i16 + ((tileRef as i8 as i16) * 16)) as usize, //signed addition
                            1 => (tileRef as usize) * 16usize, 
                            _ => panic!("Uh oh, the tile set should only be 0 or 1")
                        };


                        //since we already found the correct tile, we only need the last 3 bits of the 
                        //y-scroll register to determine where in the tile we start
                        tileAddr += ((y & 7) as usize) * 2;

                        let highBit = if (lcd.videoRAM[tileAddr + 1] & xMask) != 0 {1u8} else {0};
                        let lowBit = if (lcd.videoRAM[tileAddr] & xMask) != 0 {1u8} else {0};

                        let color = lcd.palette[((highBit * 2) + lowBit) as usize];

                        //after all this shit, finally draw the pixel
                        lcd.screenBackBuffer[lcd.currScanLine as usize][x as usize] = color; 

                        //update xMask and tile reference appropriately if we are at the end of a tile
                        match xMask {
                            1 => {
                                xMask = 0x80;
                                //the mod 32 makes sure we wrap around to the beginning of the tile map row,
                                //if need be
                                tileRefAddr = tileRefRowStart + ((tileRefAddr + 1) % 32);
                            },
                            _ => xMask >>= 1
                        };

                    }
                }
                //background not enabled
                else {
                    for x in 0..160 {
                        //just draw white
                        lcd.screenBackBuffer[lcd.currScanLine as usize][x as usize] = WHITE; 
                    }
                }



                lcd.mode = HBlank;
                lcd.modeClock = 0;

            },

            _ => {} //do nothing
        }
    }

}

