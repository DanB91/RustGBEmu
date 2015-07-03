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
    pub lcd: [[LCDPixelColor;144];160],
}

impl GPUState {
    pub fn new() -> GPUState {
        GPUState {
            mode: GPUMode::ScanOAM,
            modeClock: 0,
            currLine: 0,
            lcd: [[LCDPixelColor::White;144];160],
        }
    }
}
