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
mod gb_gpu;
mod sdl2_ttf;

use std::env;
//use std::io;
use std::thread;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{sync_channel, SyncSender};

use libc::funcs::posix88::unistd::usleep;
use libc::consts::os::posix88::EINTR;

use errno::*;

use gb_memory::*;
use gb_cpu::*;
use gb_gpu::*;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::timer::{get_performance_counter, get_performance_frequency};
use sdl2::rect::Rect;


static USAGE: &'static str= "Usage: gbemu path_to_rom";
static FONT_PATH_STR: &'static str = "res/Gamegirl.ttf";

const GAMEBOY_SCALE: u32 = 4;
const SCREEN_WIDTH: u32 = 160 * GAMEBOY_SCALE;
const SCREEN_HEIGHT: u32 = 144 * GAMEBOY_SCALE;

const SECONDS_PER_FRAME: f32 = 1f32/60f32;
const CYCLES_PER_SLEEP: u32 = 60000;

struct GameBoy {
    pub cpu: CPUState,
    pub mem: MemoryState,
    pub gpu: GPUState
}

impl GameBoy {
    fn new() -> GameBoy {
        GameBoy {
            cpu: CPUState::new(),
            mem: MemoryState::new(),
            gpu: GPUState::new()
        }
    }
}

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
fn disassemble(cpu: &CPUState, mem: &MemoryState) -> String {
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

fn step(cpu: &mut CPUState, mem: &mut MemoryState, gpu: &mut GPUState, readyScreensTx: &SyncSender<LCDScreen>) {
    use gb_gpu::GPUMode::*;

    let instructionToExecute = readByteFromMemory(mem, cpu.PC);
    
    if cpu.PC > 0xFF {
        mem.inBios = false;
    }

    let (newPC, cyclesTaken) = executeInstruction(instructionToExecute, cpu, mem); 
    cpu.PC = newPC;
    cpu.instructionCycles = cyclesTaken;
    cpu.totalCycles += cyclesTaken;

    gpu.modeClock += cyclesTaken;

    match gpu.mode {

        HBlank if gpu.modeClock >= 204 => {
            gpu.modeClock = 0;
            gpu.currLine += 1;

            //at the last line, engage VBlank and draw SDL screen
            if gpu.currLine == 143 {
                gpu.mode = VBlank;

                println!("Sending...");
                readyScreensTx.send(gpu.lcd).unwrap();
            }
            else {
                gpu.mode = ScanOAM;
            }
        },

        VBlank if gpu.modeClock >= 456 => {
            gpu.currLine += 1;

            if gpu.currLine == 153 {
                gpu.mode = ScanOAM;
                gpu.currLine = 0;
                gpu.modeClock = 0;
            }
        },

        ScanOAM if gpu.modeClock >= 80 => {
            //TODO: Draw OAM to internal screen buffer

            gpu.mode = ScanVRAM;
            gpu.modeClock = 0;
        },

        ScanVRAM if gpu.modeClock >= 172 => {
            //TODO: Draw VRAM to internal screen buffer
            gpu.mode = HBlank;
            gpu.modeClock = 0;

        },

        _ => {} //do nothing
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

    let gb = Arc::new(RwLock::new(GameBoy::new()));
    gb.write().unwrap().mem.romData = romData;

    let isRunning = Arc::new(Mutex::new(true));
    let mhz = Arc::new(RwLock::new(0f32));

    let gbLock = gb.clone();
    let isRunningLock = isRunning.clone();
    let mhzLock = mhz.clone();

    let (readyScreensTx, readyScreensRx) = sync_channel::<LCDScreen>(0);
    

    //init SDL 
    let mut sdlContext = sdl2::init().video().events().unwrap();
    sdl2_ttf::init().unwrap();
    let window = sdlContext.window("GB Emu", SCREEN_WIDTH, SCREEN_HEIGHT).position_centered().build().unwrap();
    let mut renderer = window.renderer().build().unwrap();
    let font =  sdl2_ttf::Font::from_file(Path::new(FONT_PATH_STR), 12).unwrap();

    //spawn game boy thread 
    let emuThread = thread::spawn(move || {

        let mut batchCycles = 0u32;
        let mut start = get_performance_counter();

        while *isRunningLock.lock().unwrap() {
            {
                let mut gbLock = &mut *gbLock.write().unwrap();
                step(&mut gbLock.cpu, &mut gbLock.mem, &mut gbLock.gpu, &readyScreensTx);
                batchCycles += gbLock.cpu.instructionCycles;
            }

            //sleep every several thousand cycles
            if batchCycles >= CYCLES_PER_SLEEP {
                let secsElapsed = secondsForCountRange(start, get_performance_counter());
                let targetSecs =  batchCycles as f32 / CLOCK_SPEED_HZ; 

                if secsElapsed < targetSecs {
                    let secsToSleep = targetSecs - secsElapsed;
                    sleep(secsToSleep).unwrap();
                }

                let secsElapsed = secondsForCountRange(start, get_performance_counter());
                let hz = batchCycles as f32 / secsElapsed;
                *mhzLock.write().unwrap() = hz / 1000000f32;

                batchCycles = 0;
                start = get_performance_counter();

            }
        }
    });

    let mut fps = 0f32;

    let mut shouldDisplayDebug = true;

    //I/O loop
    while *isRunning.lock().unwrap() {

        let start = get_performance_counter();

        //SDL Events
        for event in sdlContext.event_pump().poll_iter() {

            match event {
                Event::Quit{..} => *isRunning.lock().unwrap() = false,
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
        
        renderer.clear();


        //draw LCD screen
        let lcdScreen = match readyScreensRx.recv() {
            Ok(screen) => screen,
            Err(err) => panic!("A fatal error has occurred: {}", err)
        };
        println!("Received");

        let mut x = 0u32;
        let mut y  = 0u32;

        for row in &lcdScreen[..] {
            for pixel in &row[..] {

                let color = match *pixel {
                    LCDPixelColor::White => Color::RGBA(255,255,255,255),
                    LCDPixelColor::Light => Color::RGBA(170,170,170,255),
                    LCDPixelColor::Dark => Color::RGBA(85,85,85,255),
                    LCDPixelColor::Black => Color::RGBA(0,0,0,255)
                };

                renderer.set_draw_color(color);
                renderer.draw_rect(Rect::new_unwrap(x as i32 ,y as i32, GAMEBOY_SCALE, GAMEBOY_SCALE));

                x = (x + GAMEBOY_SCALE) % (row.len() as u32 * GAMEBOY_SCALE);
            }

            y += GAMEBOY_SCALE;
        }

        //display Game Boy debug stats
        if shouldDisplayDebug { 
            let toPrint: String;

            //acquire lock on gb object
            {
                let gb = gb.read().unwrap();
                let cpu = &gb.cpu;
                let mem = &gb.mem;
                let mhz = mhz.read().unwrap();

                let mut instructionToPrint = readByteFromMemory(mem, cpu.PC) as u16;

                if instructionToPrint == 0xCB {
                    instructionToPrint =  word(0xCBu8, readByteFromMemory(&mem, cpu.PC.wrapping_add(1)))
                }

                //print debug details
                toPrint = format!("{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",  
                                  format!("Current Insruction: {}\tOpcode:{:X}", disassemble(&cpu, &mem), instructionToPrint),
                                  format!("Total Cycles: {}, Cycles just executed: {}", cpu.totalCycles, cpu.instructionCycles),
                                  format!("Mhz {:.*}", 2, *mhz),
                                  format!("Currently in BIOS: {}", mem.inBios),
                                  format!("Flags: Z: {}, N: {}, H: {}, C: {}", isFlagSet(Flag::Zero, cpu.F), isFlagSet(Flag::Neg, cpu.F), isFlagSet(Flag::Half, cpu.F), isFlagSet(Flag::Carry, cpu.F)),
                                  format!("PC: {:X}\tSP: {:X}", cpu.PC, cpu.SP),
                                  format!("A: {:X}\tF: {:X}\tB: {:X}\tC: {:X}", cpu.A, cpu.F, cpu.B, cpu.C),
                                  format!("D: {:X}\tE: {:X}\tH: {:X}\tL: {:X}", cpu.D, cpu.E, cpu.H, cpu.L),
                                  format!("FPS: {}", fps));

            } //release lock


            let fontSurf =  font.render_str_blended_wrapped(&toPrint, Color::RGBA(255,0,0,255), SCREEN_WIDTH).unwrap();
            let mut fontTex = renderer.create_texture_from_surface(&fontSurf).unwrap();

            let (texW, texH) = { let q = fontTex.query(); (q.width, q.height)};
            renderer.copy(&mut fontTex, None, sdl2::rect::Rect::new(0, 0, texW, texH).unwrap());
        }

        renderer.present();

        /*
        let secsElapsed = secondsForCountRange(start, get_performance_counter());

        //60 fps
        if secsElapsed < SECONDS_PER_FRAME {
            sleep(SECONDS_PER_FRAME - secsElapsed).unwrap();
        }
        */
        fps = 1f32/secondsForCountRange(start, get_performance_counter());

    }


    emuThread.join().unwrap();

    sdl2_ttf::quit();


}



