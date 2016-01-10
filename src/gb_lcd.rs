extern crate sdl2;

use std::mem::swap; 
use self::LCDMode::*;

//Holds the state of the  screen and controller
pub struct LCDState {
    //rename to backgroundPalette
    pub palette: [PaletteColor;4], //color palette
    pub spritePalette0: [PaletteColor;4], 
    pub spritePalette1: [PaletteColor;4],
    pub videoRAM: [u8;0x2000],
    pub oam: [u8;0xA0], //sprite memory
    pub mode: LCDMode, 
    pub modeClock: u32,
    pub currScanLine: u8, //ly
    pub backgroundTileMap: u8, //which background tile map to use (0 or 1)
    pub backgroundTileSet: u8, //which background tile set to use (0 or 1)
    pub isBackgroundEnabled: bool,
    pub isEnabled: bool,
    pub isOAMEnabled: bool,

    pub scx: u8, //scroll x
    pub scy: u8, //scroll y
    pub spriteHeight: SpriteHeight, //can be 8 or 16

    pub lcdc: u8, //tells when to engage the lcdc interrupt
    pub lyc: u8,

    pub screen: LCDScreen,
    pub screenBackBuffer: LCDScreen,
}

#[derive(PartialEq, Copy, Clone)]
pub enum LCDMode {
    HBlank = 0,
    VBlank = 1, 
    ScanOAM = 2,
    ScanVRAMAndOAM = 3
}

#[derive(PartialEq, Copy, Clone)]
pub enum SpriteHeight {
    Short = 8,
    Tall = 16
}

pub type PaletteColor = sdl2::pixels::Color;
pub const WHITE: PaletteColor = sdl2::pixels::Color::RGBA(255, 255, 255, 255);
pub const LIGHT_GRAY: PaletteColor = sdl2::pixels::Color::RGBA(170,170,170,255);
pub const DARK_GRAY: PaletteColor = sdl2::pixels::Color::RGBA(85,85,85,255);
pub const BLACK: PaletteColor = sdl2::pixels::Color::RGBA(0,0,0,255);

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;


pub type LCDScreen = [[PaletteColor;SCREEN_WIDTH];SCREEN_HEIGHT]; 
pub const BLANK_SCREEN: LCDScreen = [[WHITE;SCREEN_WIDTH];SCREEN_HEIGHT];

const TILE_WIDTH: usize = 8;
const TILE_HEIGHT: usize = 8;
const BYTES_PER_TILE_ROW: usize = 2;
const BYTES_PER_TILE: usize = 16;
const TILE_MAP_WIDTH: usize = 32;
const TILE_MAP_HEIGHT: usize = 32;
const MAX_SPRITES_PER_SCANLINE: usize = 10;

const TALL_SPRITE_HEIGHT: usize = 16;
const SHORT_SPRITE_HEIGHT: usize = 8;

impl LCDState {

    pub fn new() -> LCDState {
        LCDState {

            mode: LCDMode::ScanOAM,
            modeClock: 0,
            scx: 0, //scroll x
            scy: 0, //scroll y
            spriteHeight:  SpriteHeight::Short, 
            currScanLine: 0,
            videoRAM: [0;0x2000],
            oam: [0;0xA0],
            backgroundTileMap: 0, //which tile map to use (0 or 1)
            backgroundTileSet: 0, //which tile set to use (0 or 1)
            isBackgroundEnabled: false,
            palette: [WHITE, WHITE, WHITE, WHITE], //color pallet
            spritePalette0: [WHITE, WHITE, WHITE, WHITE], 
            spritePalette1: [WHITE, WHITE, WHITE, WHITE], 
            isEnabled: false,
            isOAMEnabled: false,


            lcdc: 0,
            lyc: 0,

            screen: BLANK_SCREEN,
            screenBackBuffer: BLANK_SCREEN
        }
    }


}

struct Sprite {
    y: u8,
    x: u8,
    tileReference: u8,

    isBelowBackground: bool,
    isYFlipped: bool,
    isXFlipped: bool,
    selectedSpritePalette: SpritePalette,

    oamIndex: usize //the index in LCDState.oam that the sprite is stored in.
        //used for priority sorting
}

#[derive(PartialEq, Copy, Clone)]
enum SpritePalette {
    Palette0 = 0,
    Palette1 = 1
}

impl Sprite {
    fn new(y: u8, x: u8, tileReference: u8, flags: u8, oamIndex: usize) -> Sprite {
        let isBelowBackground = testBit!(flags, 7);  
        let isYFlipped = testBit!(flags, 6);
        let isXFlipped = testBit!(flags, 5);
        let selectedSpritePalette =
            if testBit!(flags, 4) {
                Palette1
            }
            else {
                Palette0
            };

        Sprite {
            y: y,
            x: x,
            tileReference: tileReference,

            isBelowBackground: isBelowBackground,
            isYFlipped: isYFlipped,
            isXFlipped: isXFlipped,
            selectedSpritePalette: selectedSpritePalette,

            oamIndex: oamIndex
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
enum ColorNumber {
    Color0 = 0,
    Color1 = 1,
    Color2 = 2,
    Color3 = 3
}

impl ColorNumber {
    fn fromU8(num: u8) -> ColorNumber {
        match num {
            0 => Color0,
            1 => Color1,
            2 => Color2,
            3 => Color3,
            _ => panic!("Color number should be between 0 and 3")
        }

    }
}

use self::ColorNumber::*;
use self::SpritePalette::*;
use self::SpriteHeight::*;

fn spritePaletteColorForColorNumber(colorNum: ColorNumber, sprite: &Sprite, lcd: &mut LCDState) -> PaletteColor {
    debug_assert!(colorNum != Color0, "Color0 is not a valid palette color for sprites"); //Color0 is not a valid palette color for sprites

    match sprite.selectedSpritePalette {
        Palette0 => lcd.spritePalette0[colorNum as usize],
        Palette1 => lcd.spritePalette1[colorNum as usize]
    }
}

fn backgroundPaletteColorForColorNumber(colorNum: ColorNumber, lcd: &mut LCDState) -> PaletteColor {
    lcd.palette[colorNum as usize]
}

fn getBackgroundTileReferenceStartAddress(lcd: &mut LCDState) -> usize {
    let yInPixels = lcd.scy.wrapping_add(lcd.currScanLine);

    let mut tileRefAddr = match lcd.backgroundTileMap {
        0 => 0x1800usize,  //it is 0x1800 instead of 0x9800 because this is relative to start of vram
          1 => 0x1C00usize,
          _ => panic!("Uh oh, the tile map should only be 0 or 1")
    };

    /* Tile Map:
     *
     * Each "row" is 32 bytes long where each byte is a tile reference
     * Each byte represents a 8x8 pixel tile, so each row and column are 256 pixels long
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
    tileRefAddr += (yInPixels as usize / TILE_HEIGHT) * TILE_MAP_HEIGHT; //which tile in the y dimension?

    tileRefAddr += lcd.scx as usize / TILE_WIDTH; //which tile in x dimension?

    tileRefAddr

}

fn getBackgroundTileAddressFromReferenceAddress(backgroundTileReferenceAddress: usize, lcd: &mut LCDState) -> usize {
    let yInPixels = lcd.scy.wrapping_add(lcd.currScanLine);
    let tileRef = lcd.videoRAM[backgroundTileReferenceAddress];

    //find the tile based on the tile reference
    let mut tileAddr = match lcd.backgroundTileSet {
        0 => (0x1000i16 + ((tileRef as i8 as i16) * BYTES_PER_TILE as i16)) as usize, //signed addition
        1 => (tileRef as usize) * BYTES_PER_TILE, 
        _ => panic!("Uh oh, the tile set should only be 0 or 1")
    };


    //since we already found the correct tile, we only need the last 3 bits of the 
    //y-scroll register to determine where in the tile we start
    tileAddr += ((yInPixels & 7) as usize) * BYTES_PER_TILE_ROW;

    tileAddr

}

fn colorNumberForBackgroundTileReferenceAddress(backgroundTileRefAddr: usize, scanLinePos: usize, lcd: &mut LCDState) -> ColorNumber {

    let xMask = 0x80u8 >> ((scanLinePos as usize) & 7);
    let backgroundTileAddr = getBackgroundTileAddressFromReferenceAddress(backgroundTileRefAddr, lcd);

    let highBit = if (lcd.videoRAM[backgroundTileAddr + 1] & xMask) != 0 {1u8} else {0};
    let lowBit = if (lcd.videoRAM[backgroundTileAddr] & xMask) != 0 {1u8} else {0};

    ColorNumber::fromU8((highBit * 2) + lowBit)

}

//TODO: refactor
fn colorNumberForSprite(sprite: &Sprite, posInScanLine: usize, lcd: &mut LCDState) -> ColorNumber {

    let currPixelYPostion = lcd.currScanLine as usize;
    let spriteYStart = sprite.y.wrapping_sub(16) as usize;


    let spriteXStart = sprite.x.wrapping_sub(8) as usize;
    let currPixelXPostion = posInScanLine;

    debug_assert!(currPixelXPostion >= spriteXStart);
    debug_assert!(currPixelYPostion >= spriteYStart);

    let currPixelYPostionInTile = currPixelYPostion - spriteYStart; 


    let xOffset = if sprite.isXFlipped {
        7 - (currPixelXPostion - spriteXStart)
    }
    else {
        currPixelXPostion - spriteXStart
    };


    let yOffset = if sprite.isYFlipped {
        match lcd.spriteHeight {
            //the 8 is because we only look at the first tile anyway when it is flipped
            Short if currPixelYPostion - spriteYStart < 8 => 7 - (currPixelYPostion - spriteYStart),
            Short => (currPixelYPostion - spriteYStart),
            Tall => 15 - (currPixelYPostion - spriteYStart),
        }

    }
    else {
        currPixelYPostion - spriteYStart
    };

    let xMask = 0x80u8 >> xOffset;

    match lcd.spriteHeight {
        Short => {
            if currPixelYPostionInTile < Short as usize {
                //sprites start at start of vram
                let mut tileAddr = sprite.tileReference as usize * BYTES_PER_TILE; 

                tileAddr += (yOffset as usize) * BYTES_PER_TILE_ROW;

                let highBit = if (lcd.videoRAM[tileAddr + 1] & xMask) != 0 {1u8} else {0};
                let lowBit = if (lcd.videoRAM[tileAddr] & xMask) != 0 {1u8} else {0};

                ColorNumber::fromU8((highBit * 2) + lowBit)
            }
            else {
                Color0 //transparent
            }
        }

        Tall => {
            let tileRef = if (currPixelYPostionInTile < 8 && !sprite.isYFlipped) ||
                (currPixelYPostionInTile >= 8 && sprite.isYFlipped) {
                sprite.tileReference & 0xFE
            } else {
                sprite.tileReference | 1
            };

            //sprites start at start of vram
            let mut tileAddr = tileRef as usize * BYTES_PER_TILE; 

            tileAddr += (yOffset as usize) * BYTES_PER_TILE_ROW;

            let highBit = if (lcd.videoRAM[tileAddr + 1] & xMask) != 0 {1u8} else {0};
            let lowBit = if (lcd.videoRAM[tileAddr] & xMask) != 0 {1u8} else {0};

            ColorNumber::fromU8((highBit * 2) + lowBit)

        }
    }



}

fn changeScanLine(newScanLine: u8, lcd: &mut LCDState, requestedInterrupts: &mut u8) {
    lcd.currScanLine = newScanLine;

    //NOTE: currScanLine is ly
    //if lyc == ly...
    if lcd.currScanLine == lcd.lyc {
        lcd.lcdc |= 1 << 3; //turn on lyc == ly status bit

        //request lcdc interrupt if enabled
        if (lcd.lcdc & 1 << 6) != 0 {

            *requestedInterrupts |= 1 << 1;
        }
    }
    else {
        lcd.lcdc &= !(1 << 3); //turn off lyc == ly status bit
    }

}

fn changeToNewLCDMode(newMode: LCDMode, lcd: &mut LCDState, requestedInterrupts: &mut u8) {
    lcd.mode = newMode;

    *requestedInterrupts |= match newMode {
        ScanOAM if lcd.lcdc & (1 << 5) != 0  => 1 << 1,
        HBlank if lcd.lcdc & (1 << 4) != 0  => 1 << 1,
        VBlank if lcd.lcdc & (1 << 3) != 0 => 1 << 1,

        _ => *requestedInterrupts
    };

}



pub fn stepLCD(lcd: &mut LCDState, requestedInterrupts: &mut u8, cyclesTakenOfLastInstruction: u32) {

    if lcd.isEnabled {

        //get instruction cycles of last instruction exectued
        lcd.modeClock += cyclesTakenOfLastInstruction; 

        match lcd.mode {

            HBlank if lcd.modeClock >= 204 => {
                lcd.modeClock = 0;
                changeScanLine(lcd.currScanLine + 1, lcd, requestedInterrupts);

                //at the last line...
                if lcd.currScanLine == 143 {
                    changeToNewLCDMode(VBlank, lcd, requestedInterrupts); //engage VBlank
                    swap(&mut lcd.screen, &mut lcd.screenBackBuffer); //commit fully drawn screen
                    *requestedInterrupts |= 1; //request VBlank interrupt
                }
                else {
                    lcd.mode = ScanOAM;
                }
            },

            VBlank if lcd.modeClock >= 456 => {
                lcd.modeClock = 0;
                changeScanLine(lcd.currScanLine + 1, lcd, requestedInterrupts);

                if lcd.currScanLine == 153 {
                    changeToNewLCDMode(ScanOAM, lcd, requestedInterrupts); 
                    changeScanLine(0, lcd, requestedInterrupts);

                }
            },

            ScanOAM if lcd.modeClock >= 80 => {
                //TODO: Draw OAM to internal screen buffer

                changeToNewLCDMode(ScanVRAMAndOAM, lcd, requestedInterrupts); 
                lcd.modeClock = 0;
            },

            ScanVRAMAndOAM if lcd.modeClock >= 172 => {
                //TODO: Draw VRAM to internal screen buffer


                let mut backgroundTileRefAddr = getBackgroundTileReferenceStartAddress(lcd);
                let backgroundTileRefRowStart = backgroundTileRefAddr - (lcd.scx as usize / TILE_WIDTH); 


                let mut spritesSortedByPriority: Vec<Sprite> = vec![];
                let mut numSpritesToDraw = 0;

                if lcd.isOAMEnabled {
                    //get sprites to draw for this scan line 
                    let mut i = 0;
                    while i < lcd.oam.len() {
                        //sprite location is lower right hand corner
                        //so x and y coords are offset by 8 and 16 respectively

                        let spriteY = lcd.oam[i];
                        let spriteX = lcd.oam[i+1];

                        //x coordinates explicitly ignored since even though sprites outside of the
                        //screen are not drawn, they do affect priority
                        if lcd.currScanLine < spriteY &&
                            lcd.currScanLine >= spriteY.wrapping_sub(16) {

                                spritesSortedByPriority.push(
                                    Sprite::new(spriteY, spriteX, lcd.oam[i+2], lcd.oam[i+3], i)
                                    );
                            }

                        i += 4;

                    }

                    //sort sprites by priority (last element is lowest priority)
                    spritesSortedByPriority.sort_by(|left, right| {
                        if left.x != right.x {
                            left.x.cmp(&right.x)
                        }
                        else {
                            left.oamIndex.cmp(&right.oamIndex)
                        }
                    });

                    numSpritesToDraw = if spritesSortedByPriority.len() < MAX_SPRITES_PER_SCANLINE {
                        spritesSortedByPriority.len()
                    }
                    else {
                        MAX_SPRITES_PER_SCANLINE
                    };

                }

                //can only draw at most 10 sprites per scanline
                let spritesToDraw = &spritesSortedByPriority[0..numSpritesToDraw];


                for posInScanLine in 0..SCREEN_WIDTH {

                    let mut spriteColorNum = Color0;
                    let mut spriteToDraw = None;

                    for sprite in spritesToDraw {

                        if (posInScanLine as u8) < sprite.x && 
                            (posInScanLine as u8) >= sprite.x.wrapping_sub(8) {

                                spriteColorNum = colorNumberForSprite(&sprite, posInScanLine, lcd);

                                //NOTE: Color0 indicates transparent in this case.  If the sprite pixel is
                                //not transparent, then we found the sprite to draw since we
                                //already sorted by priority
                                if spriteColorNum != Color0 {
                                    spriteToDraw = Some(sprite);
                                    break;
                                }
                            }
                    }


                    let backgroundColorNum = if lcd.isBackgroundEnabled {
                        colorNumberForBackgroundTileReferenceAddress(backgroundTileRefAddr, posInScanLine, lcd) 
                    }
                    else {
                        Color0
                    };

                    let colorToDraw = 
                        match spriteToDraw  {
                            Some(sprite) => {
                                if sprite.isBelowBackground {
                                    if backgroundColorNum != Color0 {
                                        backgroundPaletteColorForColorNumber(backgroundColorNum, lcd)
                                    }
                                    else if spriteColorNum != Color0 {
                                        spritePaletteColorForColorNumber(spriteColorNum, &sprite, lcd)
                                    }
                                    else {
                                        WHITE
                                    }
                                }
                                else {
                                    if spriteColorNum != Color0 {
                                        spritePaletteColorForColorNumber(spriteColorNum, &sprite, lcd)
                                    }
                                    else {
                                        backgroundPaletteColorForColorNumber(backgroundColorNum, lcd)
                                    }

                                }
                            }
                            None => backgroundPaletteColorForColorNumber(backgroundColorNum, lcd)
                        }; 

                    //after all this shit, finally draw the pixel
                    lcd.screenBackBuffer[lcd.currScanLine as usize][posInScanLine as usize] = colorToDraw;

                    if posInScanLine % TILE_WIDTH == 7 {
                        backgroundTileRefAddr = backgroundTileRefRowStart + ((backgroundTileRefAddr + 1) % TILE_MAP_HEIGHT);
                    }

                }

                changeToNewLCDMode(HBlank, lcd, requestedInterrupts); 
                lcd.modeClock = 0;

            },

            _ => {} //do nothing
        }
    }

}

