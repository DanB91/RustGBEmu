#![allow(non_snake_case)]
#![allow(dead_code)]

#[macro_use]
extern crate bitflags;
extern crate sdl2;
extern crate libc;
extern crate sdl2_sys;

extern crate errno;

mod gb_memory;
mod gb_cpu;
mod sdl2_ttf;

use std::env;
use std::path::Path;

use libc::funcs::posix88::unistd::usleep;
use libc::consts::os::posix88::EINTR;

use errno::*;

use gb_memory::*;
use gb_cpu::*;

use sdl2::render::Renderer;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::timer::{get_performance_counter, get_performance_frequency};
use sdl2::rect::Rect;

use sdl2_ttf::Font;

use gb_memory::LCDMode::*;

static USAGE: &'static str= "Usage: gbemu path_to_rom";
static FONT_PATH_STR: &'static str = "res/Gamegirl.ttf";

const GAMEBOY_SCALE: u32 = 2;
const SCREEN_WIDTH: u32 = 160 * GAMEBOY_SCALE;
const SCREEN_HEIGHT: u32 = 144 * GAMEBOY_SCALE;

const SECONDS_PER_FRAME: f32 = 1f32/60f32;
const CYCLES_PER_SLEEP: u32 = 60000;


fn getROMFileName() -> Result<String, &'static str> {

    let mut i = 0;
    let mut retStr = Err(USAGE);
    for arg in env::args() {
        retStr = match i {
            1 => Ok(arg),
            _ => Err(USAGE), 
        };
        i += 1;

    }

    retStr
}

//TODO: finish disassembler
fn disassemble(cpu: &CPUState, mem: &MemoryMapState) -> String {
    let instruction = readByteFromMemory(mem, cpu.PC);

    macro_rules! nextByte {
        () => (readByteFromMemory(mem, cpu.PC.wrapping_add(1)))
    }

    macro_rules! nextWord {
        () => (readWordFromMemory(mem, cpu.PC.wrapping_add(1)))
    }
    match instruction {
        0x0 => format!("NOP"),
        0x1 => format!("LD BC ${:X}", nextWord!()),
        0x2 => format!("LD (BC) A"),
        0x3 => format!("INC BC"),
        0x4 => format!("INC B"),
        0x5 => format!("DEC B"),
        0x6 => format!("LD B ${:X}", nextByte!()),
        0x7 => format!("RLCA"),
        0x8 => format!("LD (${:X}), SP", nextWord!()),
        0x20 => format!("JR NZ Addr_{:X}", 
                        (cpu.PC as i16).wrapping_add(nextByte!() as i8 as i16).wrapping_add(2)),
                        _ => format!("")
    }

}

//Returns number of seconds for a given performance count range
fn secondsForCountRange(start: u64, end: u64) -> f32 {
    ((end as f64 - start as f64) / get_performance_frequency() as f64) as f32
}

fn sleep(secsToSleep: f32) -> Result<(), String> {
    //NOTE(DanB): .005 denotes the amount we will spin manually since
    //      nanosleep is not 100% accurate 

    let start = get_performance_counter();
    let newSecsToSleep = secsToSleep - 0.005f32;

    if newSecsToSleep < 0f32 {
        while secondsForCountRange(start, get_performance_counter()) < secsToSleep {
        }

        return Ok(());
    }


    unsafe {
        let microSecsToSleep = (newSecsToSleep * 1000000f32) as u32;

        if usleep(microSecsToSleep) == -1{
            let error = errno();
            match error {
                Errno(errnum) if errnum == EINTR => {
                    println!("Warning: thread interrupted");
                    Ok(())
                },
                _ => Err(format!("Could not sleep for {} ms.  Error: {}", microSecsToSleep, error))
            }
        }
        else {
            //spin for the rest of the .005 seconds
            while secondsForCountRange(start, get_performance_counter()) < secsToSleep {
            }

            Ok(())
        }

    }

}


//TODO(DanB): Handle unwrap
fn main() {

    //parse cmd args
    let fileName = match getROMFileName() {
        Ok(fileName) => fileName,
        Err(err) => {
            println!("{}", err);
            return
        }
    };


    //load ROM
    let romData = match openROM(&fileName[..]) {
        Ok(data) => data,
        Err(err) => panic!("{}", err)
    };

    let mut cpu = CPUState::new();
    let mut mem = MemoryMapState::new();
    let mut lcdScreen = [[WHITE;160];144];

    mem.romData = romData;

    let mut isRunning = true;
    let mut mhz = 0f32;

    //init SDL 
    let mut sdlContext = sdl2::init().video().events().unwrap();
    sdl2_ttf::init().unwrap();
    let window = sdlContext.window("GB Emu", SCREEN_WIDTH, SCREEN_HEIGHT).position_centered().build().unwrap();
    let mut renderer = window.renderer().build().unwrap();
    let font =  Font::from_file(Path::new(FONT_PATH_STR), 12).unwrap();


    let mut fps = 0f32;

    let mut shouldDisplayDebug = false;

    //main loop
    while isRunning {

        let start = get_performance_counter();

        //SDL Events
        for event in sdlContext.event_pump().poll_iter() {

            match event {
                Event::Quit{..} => isRunning = false,
                Event::KeyDown{keycode: keyOpt, ..} => {
                    match keyOpt {
                        Some(key) => {

                            if key == Keycode::D {
                                shouldDisplayDebug = !shouldDisplayDebug;
                            }
                        },
                        None => {}
                    }
                }
                _ => {}
            }   
        }


        //run several thousand game boy cycles or so 
        let mut batchCycles = 0u32;

        //one Game Boy "step" per iteration
        while batchCycles < CYCLES_PER_SLEEP{
            //step CPU
            let instructionToExecute = readByteFromMemory(&mut mem, cpu.PC);

            let (newPC, cyclesTaken) = executeInstruction(instructionToExecute, &mut cpu, &mut mem); 
            cpu.PC = newPC;
            cpu.instructionCycles = cyclesTaken;
            cpu.totalCycles.wrapping_add(cyclesTaken);

            //step GPU
            if mem.isLCDEnabled {
                mem.lcdModeClock += cyclesTaken;
                match mem.lcdMode {

                    HBlank if mem.lcdModeClock >= 204 => {
                        mem.lcdModeClock = 0;
                        mem.currScanLine += 1;

                        //at the last line, engage VBlank and draw SDL screen
                        if mem.currScanLine == 143 {
                            mem.lcdMode = VBlank;
                        }
                        else {
                            mem.lcdMode = ScanOAM;
                        }
                    },

                    VBlank if mem.lcdModeClock >= 456 => {
                        mem.currScanLine += 1;
                        mem.lcdModeClock = 0;

                        if mem.currScanLine == 153 {
                            mem.lcdMode = ScanOAM;
                            mem.currScanLine = 0;

                        }
                    },

                    ScanOAM if mem.lcdModeClock >= 80 => {
                        //TODO: Draw OAM to internal screen buffer


                        mem.lcdMode = ScanVRAM;
                        mem.lcdModeClock = 0;
                    },

                    ScanVRAM if mem.lcdModeClock >= 172 => {
                        //TODO: Draw VRAM to internal screen buffer

                        let y = mem.lcdSCY.wrapping_add(mem.currScanLine);

                        let mut tileRefAddr = match mem.backgroundTileMap {
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

                        tileRefAddr += mem.lcdSCX as usize / 8; //which tile in x dimension?

                        //the x pixel is gotten by shifting a mask of the form 100000
                        let mut xMask = 0x80u8 >> (mem.lcdSCX & 7);

                        for x in 0..160 {

                            let tileRef = mem.videoRAM[tileRefAddr];

                            //find the tile based on the tile reference
                            let mut tileAddr = match mem.backgroundTileSet {
                                0 => (0x1000i16 + ((tileRef as i8 as i16) * 16)) as usize, //signed addition
                                1 => (tileRef as usize) * 16usize, 
                                _ => panic!("Uh oh, the tile set should only be 0 or 1")
                            };


                            //since we already found the correct tile, we only need the last 3 bits of the 
                            //y-scroll register to determine where in the tile we start
                            tileAddr += ((y & 7) as usize) * 2;

                            let highBit = if (mem.videoRAM[tileAddr + 1] & xMask) != 0 {1u8} else {0};
                            let lowBit = if (mem.videoRAM[tileAddr] & xMask) != 0 {1u8} else {0};

                            let color = mem.palette[((highBit * 2) + lowBit) as usize];

                            //after all this shit, finally draw the pixel
                            lcdScreen[mem.currScanLine as usize][x as usize] = color; 

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



                        mem.lcdMode = HBlank;
                        mem.lcdModeClock = 0;

                    },

                    _ => {} //do nothing
                }
            }

            batchCycles += cpu.instructionCycles;
        }

        renderer.clear();

        //draw clear screen if lcd is disabled
        if mem.isLCDEnabled {        

            //draw LCD screen
            let mut x = 0u32;
            let mut y  = 0u32;

            for row in &lcdScreen[..] {
                for color in &row[..] {

                    renderer.set_draw_color(*color);
                    renderer.fill_rect(Rect::new_unwrap(x as i32 ,y as i32, GAMEBOY_SCALE, GAMEBOY_SCALE));

                    x = (x + GAMEBOY_SCALE) % (row.len() as u32 * GAMEBOY_SCALE);
                }

                y += GAMEBOY_SCALE;
            }

        }
        else {
            renderer.set_draw_color(WHITE);
            renderer.fill_rect(Rect::new_unwrap(0,0, SCREEN_WIDTH, SCREEN_HEIGHT));

        }

        //display Game Boy debug stats
        if shouldDisplayDebug { 
            let toPrint: String;

            let mut instructionToPrint = readByteFromMemory(&mut mem, cpu.PC) as u16;

            if instructionToPrint == 0xCB {
                instructionToPrint =  word(0xCBu8, readByteFromMemory(&mut mem, cpu.PC.wrapping_add(1)))
            }

            //print debug details
            toPrint = format!("{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",  
                              format!("Opcode:{:X}", instructionToPrint),
                              format!("Total Cycles: {}, Cycles just executed: {}", cpu.totalCycles, cpu.instructionCycles),
                              format!("Mhz {:.*}", 2, mhz),
                              format!("Currently in BIOS: {}", mem.inBios),
                              format!("Flags: Z: {}, N: {}, H: {}, C: {}", isFlagSet(Flag::Zero, cpu.F), isFlagSet(Flag::Neg, cpu.F), isFlagSet(Flag::Half, cpu.F), isFlagSet(Flag::Carry, cpu.F)),
                              format!("PC: {:X}\tSP: {:X}", cpu.PC, cpu.SP),
                              format!("A: {:X}\tF: {:X}\tB: {:X}\tC: {:X}", cpu.A, cpu.F, cpu.B, cpu.C),
                              format!("D: {:X}\tE: {:X}\tH: {:X}\tL: {:X}", cpu.D, cpu.E, cpu.H, cpu.L),
                              format!("FPS: {}", fps));



            let fontSurf =  font.render_str_blended_wrapped(&toPrint, sdl2::pixels::Color::RGBA(255,0,0,255), SCREEN_WIDTH).unwrap();
            let mut fontTex = renderer.create_texture_from_surface(&fontSurf).unwrap();

            let (texW, texH) = { let q = fontTex.query(); (q.width, q.height)};
            renderer.copy(&mut fontTex, None, sdl2::rect::Rect::new(0, 0, texW, texH).unwrap());
        }
        
        renderer.present();

        let secsElapsed = secondsForCountRange(start, get_performance_counter());
        let targetSecs =  batchCycles as f32 / CLOCK_SPEED_HZ; 

        if secsElapsed < targetSecs {
            let secsToSleep = targetSecs - secsElapsed;
            sleep(secsToSleep).unwrap();
        }

        //TODO: clock speed and fps lag one frame behind
        let secsElapsed = secondsForCountRange(start, get_performance_counter());
        let hz = batchCycles as f32 / secsElapsed;
        mhz = hz / 1000000f32;
        fps = 1f32/secsElapsed;

    }



    sdl2_ttf::quit();


}


