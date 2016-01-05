extern crate sdl2;

use std::path::Path;

use sdl2_ttf;
use sdl2::render::Renderer;
use sdl2_ttf::Font;

use gb_util::*;
use gb_cpu::*;
use gb_memory::*;
use gb_gameboy::*;
use gb_lcd::*;
use std::fs::File;
use std::io::Result;
use std::io::Write;

static FONT_PATH_STR: &'static str = "res/Gamegirl.ttf";


pub struct DebugInfo {
    pub mhz: f32,
    pub fps: f32,
    pub isPaused: bool,
    pub mouseX: u32,
    pub mouseY: u32,
    pub colorMouseIsOn: &'static str, 

    pub drawHeight: u32,
    pub drawWidth: u32,
    pub drawPosX: i32,
    pub drawPosY: i32,
    
    font: Font

}

//TODO: figure out font proportions
pub fn initDebug(xpos: i32, ypos: i32, debugWidth: u32, debugHeight: u32) -> DebugInfo {
    sdl2_ttf::init().unwrap();
    let font =  Font::from_file(Path::new(FONT_PATH_STR), debugHeight as i32/16).unwrap();

    DebugInfo {
        fps: 0.,
        isPaused: false,
        mhz: 0.,
        mouseX: 0,
        mouseY: 0,
        colorMouseIsOn: "",

        drawHeight: debugHeight,
        drawWidth: debugWidth,
        drawPosX: xpos,
        drawPosY: ypos,

        font: font
    }
}

pub fn drawDebugInfo(dbg: &DebugInfo, gb: &GameBoyState, renderer: &mut Renderer) {
    let toPrint: String;

    let mut instructionToPrint = readByteFromMemory(&gb.mem, gb.cpu.PC) as u16;

    if instructionToPrint == 0xCB {
        instructionToPrint =  word(0xCBu8, readByteFromMemory(&gb.mem, gb.cpu.PC.wrapping_add(1)))
    }
    //print debug details
    toPrint = format!("{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",  
                      format!("Opcode:{:X}", instructionToPrint),
                      format!("Total Cycles: {}, Cycles just executed: {}", gb.cpu.totalCycles, gb.cpu.instructionCycles),
                      format!("Mhz {:.*}", 2, dbg.mhz),
                      format!("Currently in BIOS: {}", gb.mem.inBios),
                      format!("Flags: Z: {}, N: {}, H: {}, C: {}", isFlagSet(Flag::Zero, gb.cpu.F), isFlagSet(Flag::Neg, gb.cpu.F), isFlagSet(Flag::Half, gb.cpu.F), isFlagSet(Flag::Carry, gb.cpu.F)),
                      format!("PC: {:X}\tSP: {:X}", gb.cpu.PC, gb.cpu.SP),
                      format!("A: {:X}\tF: {:X}\tB: {:X}\tC: {:X}", gb.cpu.A, gb.cpu.F, gb.cpu.B, gb.cpu.C),
                      format!("D: {:X}\tE: {:X}\tH: {:X}\tL: {:X}", gb.cpu.D, gb.cpu.E, gb.cpu.H, gb.cpu.L),
                      format!("SCX: {}, SCY: {}", gb.mem.lcd.scx, gb.mem.lcd.scy),
                      format!("FPS: {}, Paused: {}", dbg.fps, dbg.isPaused),
                      format!("Mouse X: {}, Mouse Y: {}", dbg.mouseX, dbg.mouseY),
                      format!("Color Mouse is on: {}", dbg.colorMouseIsOn),
                      format!("DIV: {:X}, TIMA: {:X}, TMA: {:X}, Timer On: {}", gb.mem.divider, gb.mem.timerCounter, gb.mem.timerModulo, gb.mem.isTimerEnabled)); 



    let fontSurf =  dbg.font.render_str_blended_wrapped(&toPrint, sdl2::pixels::Color::RGBA(255,0,0,255), dbg.drawWidth).unwrap();
    let mut fontTex = renderer.create_texture_from_surface(&fontSurf).unwrap();

    let (texW, texH) = { let q = fontTex.query(); (q.width, q.height)};
    renderer.copy(&mut fontTex, None, sdl2::rect::Rect::new(dbg.drawPosX, dbg.drawPosY, texW, texH).unwrap());

}

pub fn dumpGameBoyState(gb: &GameBoyState, fileName: &str) -> Result<()> {
   let mut f = try!(File::create(fileName));
   let mut toPrint = format!("{}\n{}\n{}\n{}\n{}\n{}\n", 
                      format!("Currently in BIOS: {}", gb.mem.inBios),
                      format!("Flags: Z: {}, N: {}, H: {}, C: {}", isFlagSet(Flag::Zero, gb.cpu.F), isFlagSet(Flag::Neg, gb.cpu.F), isFlagSet(Flag::Half, gb.cpu.F), isFlagSet(Flag::Carry, gb.cpu.F)),
                      format!("PC: {:X}\tSP: {:X}", gb.cpu.PC, gb.cpu.SP),
                      format!("A: {:X}\tF: {:X}\tB: {:X}\tC: {:X}", gb.cpu.A, gb.cpu.F, gb.cpu.B, gb.cpu.C),
                      format!("D: {:X}\tE: {:X}\tH: {:X}\tL: {:X}", gb.cpu.D, gb.cpu.E, gb.cpu.H, gb.cpu.L),
                      format!("SCX: {}, SCY: {}", gb.mem.lcd.scx, gb.mem.lcd.scy));


   toPrint = format!("{}\n\nMemory Info\n{}\n\nLCD Info\n{}", toPrint, memDebugInfo(&gb.mem), lcdDebugInfo(&gb.mem.lcd));



   try!(f.write_all(&toPrint.into_bytes()[..]));

   Ok(())
}

fn memDebugInfo(mem: &MemoryMapState) -> String {
    let mut toPrint = "Working Ram\n".to_string();
    let mut byteRow: Vec<String> = vec![];

    //Working RAM
    for (i, row) in mem.workingRAM.chunks(8).enumerate() {
        for byte in row {
            byteRow.push(format!("{:2X}", byte));
        }

        let rowStr = byteRow.join(",");
        toPrint = format!("{}{:X}\t{}\n", toPrint, (i * 8) + 0xC000, rowStr);
        byteRow.clear();

    }

    toPrint = toPrint + "\nROM:\n";

    //ROM 
    for i in 0..0x1000 {
        for byte in 0..8 {
            byteRow.push(format!("{:2X}", readByteFromMemory(mem, byte * (i + 1))));
        }
        let rowStr = byteRow.join(",");
        toPrint = format!("{}{:X}\t{}\n", toPrint, (i * 8), rowStr);
        byteRow.clear();

    }

    toPrint
}

fn lcdDebugInfo(lcd: &LCDState) -> String {
   let mut toPrint = String::new();


   //print background tiles
   let mut tileNum = 0u16;
   let mut tile = String::new();

   for (lineIndex, bytePair) in lcd.videoRAM.chunks(2).enumerate() {
       let mut line = String::new();
       let mut mask = 0x80u8; 
       for _ in 0..8 {
           let highBit = if bytePair[1] & mask != 0 {1} else {0};
           let lowBit = if bytePair[0] & mask != 0 {1} else {0};
           let colorNum = (highBit * 2) + lowBit;

           let asciiPixel = match colorNum {
               0 => ".", //white
               1 => "!", //light gray
               2 => "*", //dark gray
               3 => "#", //black
               _ => panic!("This is mathematically impossible")
           };

           line = line + asciiPixel;

           mask >>= 1; 
       }


       tile = format!("{}{}\t{:02X}{:02X}\n", tile, line, bytePair[1], bytePair[0]);

       if (lineIndex % 8) == 7 {
           toPrint = format!("{}\n{:X}:\n{}", toPrint, tileNum, tile);
           tile = String::new();
           tileNum += 1;

       }
       

   }


   let mut oamStr = String::new() + "OAM Section:";
   for (byteNum, byte) in lcd.oam.iter().enumerate() {
       if byteNum % 4 == 0 {
           oamStr = oamStr + "\n";
       }
       oamStr = format!("{}{:X},", oamStr, byte)
   }

   toPrint = format!("{}\n\n{}", toPrint, oamStr);



   toPrint
}

pub fn debugQuit() {
    sdl2_ttf::quit();
}
