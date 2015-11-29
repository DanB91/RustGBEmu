#![allow(non_snake_case)]
#![allow(dead_code)]

extern crate sdl2;
extern crate libc;
extern crate errno;


#[macro_use]
extern crate gbEmu;


use std::env;

use libc::usleep;
use libc::EINTR;

use errno::*;

use gbEmu::gb_gameboy::*;
use gbEmu::gb_memory::*;
use gbEmu::gb_cpu::*;
use gbEmu::gb_lcd::*;
use gbEmu::gb_joypad::*;
use gbEmu::gb_debug::*;

use sdl2::event::*;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::*;


static USAGE: &'static str= "Usage: gbemu path_to_rom";

const GAMEBOY_SCALE: u32 = 4;
const SCREEN_WIDTH: u32 = 160 * GAMEBOY_SCALE;
const SCREEN_HEIGHT: u32 = 144 * GAMEBOY_SCALE;

//place to put debug area
const DEBUG_POS_Y: i32 = SCREEN_HEIGHT as i32;


const SECONDS_PER_FRAME: f32 = 1f32/60f32;
const CYCLES_PER_SLEEP: u32 = 60000;




struct ProgramState {
    shouldDisplayDebug: bool,
    isPaused: bool,
    isRunning: bool,

    gb: Box<GameBoyState>,
}

impl ProgramState {
    fn new() -> ProgramState {
        ProgramState {
            shouldDisplayDebug: false,
            isPaused: false,
            isRunning: true,

            gb: Box::new(GameBoyState::new())
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

    let mut prg = ProgramState::new();
    let mut gb = &mut *prg.gb;


    gb.mem.romData = romData;

    //init SDL 
    let sdlContext = sdl2::init().unwrap();

    let videoSubsystem = sdlContext.video().unwrap();
    let timer = sdlContext.timer().unwrap();
    let mut eventPump = sdlContext.event_pump().unwrap();

    let mut mainWindowHeight = SCREEN_HEIGHT;
    let mainWindowWidth = SCREEN_WIDTH;
    let mainWindow = videoSubsystem.window("GB Emu", mainWindowWidth, mainWindowHeight).position_centered().build().unwrap();
    let mainWindowID = mainWindow.id();
    let mut renderer = mainWindow.renderer().build().unwrap();


    //init debug screen
    let mut dbg = initDebug(0, SCREEN_HEIGHT as i32, SCREEN_WIDTH, SCREEN_HEIGHT / 4);

    //main loop
    while prg.isRunning {
        //get the start time to calculate time
        let start = timer.performance_counter();

        //TODO: Refactor event handling
        //SDL Events
        for event in eventPump.poll_iter() {

            match event {
                Event::Quit{..} => prg.isRunning = false,

                Event::Window{win_event_id, ..} => {
                    //TODO: close debug window, don't just quit app
                    match win_event_id {
                        WindowEventId::Close => {
                            prg.isRunning = false;
                        }

                        _ => {}
                    }
                },

            
                Event::KeyUp{keycode: keyOpt, ..} => {
                    match keyOpt {

                        Some(key) => {

                            match key {
                                Keycode::Up => gb.mem.joypad.up = ButtonState::Up,
                                Keycode::Down => gb.mem.joypad.down = ButtonState::Up,
                                Keycode::Left => gb.mem.joypad.left = ButtonState::Up,
                                Keycode::Right => gb.mem.joypad.right = ButtonState::Up,
                                
                                Keycode::X => gb.mem.joypad.a = ButtonState::Up,
                                Keycode::Z => gb.mem.joypad.b = ButtonState::Up,
                                Keycode::Return => gb.mem.joypad.start = ButtonState::Up,
                                Keycode::RShift => gb.mem.joypad.select = ButtonState::Up,
                                _ => {}
                            }
                        }

                        None => {}

                    }
                }


                Event::KeyDown{keycode: keyOpt, repeat: isRepeat, ..} => {
                    match keyOpt {
                        Some(key) => {
                            match key {
                                Keycode::D => {
                                    if !isRepeat {
                                        prg.shouldDisplayDebug = !prg.shouldDisplayDebug;

                                        mainWindowHeight = if prg.shouldDisplayDebug {
                                             SCREEN_HEIGHT + dbg.drawHeight
                                        }
                                        else {
                                            SCREEN_HEIGHT
                                        };

                                        renderer.window_mut().unwrap().set_size(mainWindowWidth, mainWindowHeight);
                                    }
                                } 
                                Keycode::P => {
                                    if !isRepeat {
                                        prg.isPaused = !prg.isPaused;
                                    }
                                },

                                Keycode::U => {
                                    if !isRepeat {
                                        match dumpGameBoyState(gb) {
                                           Ok(_) => { 
                                               println!("Wrote debug file");
                                           },
                                           Err(err) => {
                                               println!("{}", err);
                                           }
                                        }
                                    }
                                }

                                Keycode::Up => gb.mem.joypad.up = ButtonState::Down,
                                Keycode::Down => gb.mem.joypad.down = ButtonState::Down,
                                Keycode::Left => gb.mem.joypad.left = ButtonState::Down,
                                Keycode::Right => gb.mem.joypad.right = ButtonState::Down,

                                Keycode::X => gb.mem.joypad.a = ButtonState::Down,
                                Keycode::Z => gb.mem.joypad.b = ButtonState::Down,
                                Keycode::Return => gb.mem.joypad.start = ButtonState::Down,
                                Keycode::RShift => gb.mem.joypad.select = ButtonState::Down,
                                _ => {}
                            }
                        }

                        None => {}
                    }
                },

                Event::MouseMotion{x, y, window_id, ..} if window_id == mainWindowID => {

                    let gameBoyXPixel = (x as u32 / GAMEBOY_SCALE) as usize; 
                    let gameBoyYPixel = (y as u32 / GAMEBOY_SCALE) as usize; 

                    if gameBoyYPixel < gb.mem.lcd.screen.len() &&
                        gameBoyXPixel < gb.mem.lcd.screen[0].len() {
                            dbg.mouseX = gameBoyXPixel as u32;
                            dbg.mouseY = gameBoyYPixel as u32;

                            let pixel = gb.mem.lcd.screen[gameBoyYPixel][gameBoyXPixel];

                            dbg.colorMouseIsOn = match pixel {
                                WHITE => "White",
                                LIGHT_GRAY => "Light Gray",
                                DARK_GRAY => "Dark Gray",
                                BLACK => "Black",
                                _ => "lolwut"

                            }

                    };

                },

                _ => {}
            }   

        }

        let mut batchCycles = 0u32;

        //------------------------step emulator-------------------------------
        if !prg.isPaused {
            //run several thousand game boy cycles or so 
            while batchCycles < CYCLES_PER_SLEEP{
                stepCPU(&mut gb.cpu, &mut gb.mem);

                stepLCD(&mut gb.mem.lcd, &mut gb.mem.requestedInterrupts, gb.cpu.instructionCycles);
                batchCycles += gb.cpu.instructionCycles;
            } 
        }
        //--------------------------------------------------------------------



        //--------------------draw GB screen-----------------------------------
        renderer.clear();
        renderer.set_draw_color(WHITE);
        renderer.fill_rect(Rect::new_unwrap(0,0, mainWindowWidth, mainWindowHeight));

        //draw clear screen if lcd is disabled
        if gb.mem.lcd.isEnabled {        

            //draw LCD screen
            let mut x = 0u32;
            let mut y  = 0u32;

            for row in &gb.mem.lcd.screen[..] {
                for color in &row[..] {

                    renderer.set_draw_color(*color);
                    renderer.fill_rect(Rect::new_unwrap(x as i32 ,y as i32, GAMEBOY_SCALE, GAMEBOY_SCALE));

                    x = (x + GAMEBOY_SCALE) % (row.len() as u32 * GAMEBOY_SCALE);
                }

                y += GAMEBOY_SCALE;
            }

        }
        if gb.mem.inBios {
            println!("PC: {:X} SCY: {}  B: {}  D: {}", gb.cpu.PC, gb.mem.lcd.scy, gb.cpu.B, gb.cpu.D);

        }
        //------------------------------------------------------------------------

        //--------------------draw debug screen-----------------------------------
        
        if prg.shouldDisplayDebug {
            drawDebugInfo(&dbg, gb,  &mut renderer);
        }



        //---------------------------------------------------------------------

        renderer.present();

        let secsElapsed = secondsForCountRange(start, timer.performance_counter(), &timer);
        let targetSecs =  batchCycles as f32 / CLOCK_SPEED_HZ; 

        if secsElapsed < targetSecs {
            let secsToSleep = targetSecs - secsElapsed;
            sleep(secsToSleep, &timer).unwrap();
        }

        //TODO: clock speed and dbg.fps lag one frame behind
        let secsElapsed = secondsForCountRange(start, timer.performance_counter(), &timer);
        let hz = batchCycles as f32 / secsElapsed;
        dbg.mhz = hz / 1000000f32;
        dbg.fps = 1f32/secsElapsed;
        dbg.isPaused = prg.isPaused;


    }

    debugQuit();

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
fn secondsForCountRange(start: u64, end: u64, timer: &TimerSubsystem) -> f32 {
    ((end as f64 - start as f64) / timer.performance_frequency() as f64) as f32
}

fn sleep(secsToSleep: f32, timer: &TimerSubsystem) -> Result<(), String> {
    //NOTE(DanB): .005 denotes the amount we will spin manually since
    //      usleep is not 100% accurate 

    let start = timer.performance_counter();
    let newSecsToSleep = secsToSleep - 0.005f32;

    if newSecsToSleep < 0f32 {
        while secondsForCountRange(start, timer.performance_counter(), timer) < secsToSleep {
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
            while secondsForCountRange(start, timer.performance_counter(), timer) < secsToSleep {
            }

            Ok(())
        }

    }

}
