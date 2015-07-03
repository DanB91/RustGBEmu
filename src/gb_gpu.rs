pub enum GPUMode {
    HBlank = 0,
    VBlank, 
    ScanOAM,
    ScanVRAM
}

pub type LCDScreen = [[LCDPixelColor;144];160]; 

#[derive(Copy, Clone)]
pub enum LCDPixelColor {
    White,
    Light,
    Dark,
    Black
}

pub struct GPUState {
    pub mode: GPUMode,
    pub modeClock: u32,
    pub currLine: u32,
    pub lcdInProgress: LCDScreen, //the screen currently being constructed
    pub readyLCD: LCDScreen, //the screen ready to be flipped to SDL
}

impl GPUState {
    pub fn new() -> GPUState {
        GPUState {
            mode: GPUMode::ScanOAM,
            modeClock: 0,
            currLine: 0,
            lcdInProgress: [[LCDPixelColor::White;144];160], 
            readyLCD: [[LCDPixelColor::White;144];160], 
        }
    }
}
