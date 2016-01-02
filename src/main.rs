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

const WINDOW_WIDTH: u32 = SCREEN_WIDTH as u32 * GAMEBOY_SCALE;
const WINDOW_HEIGHT: u32 = SCREEN_HEIGHT as u32 * GAMEBOY_SCALE;

//place to put debug area
const DEBUG_POS_Y: i32 = WINDOW_HEIGHT as i32;


const SECONDS_PER_FRAME: f32 = 1f32/60f32;
const CYCLES_PER_SLEEP: u32 = 60000;


struct ProgramState {
    shouldDisplayDebug: bool,
    isPaused: bool,
    isRunning: bool,

    shouldSkipBootScreen: bool,
    romFileName: String,

    gb: Box<GameBoyState>,
}

impl ProgramState {
    fn new(romFileName: String, shouldSkipBootScreen: bool) -> ProgramState {
        ProgramState {
            shouldDisplayDebug: false,
            isPaused: false,
            isRunning: true,

            shouldSkipBootScreen: shouldSkipBootScreen,
            romFileName: romFileName,

            gb: Box::new(GameBoyState::new())
        }
    }
}

fn printUsageAndExit() -> ! {
    println!("{}", USAGE);
    std::process::exit(1)
}

//TODO: learn life time
fn parseArgs() -> ProgramState {
    let args = env::args();

    let mut romFileName = None;
    let mut shouldSkipBootScreen = false;

    if args.len() > 1 {

        for arg in args {
            match &*arg {
                "-s" => shouldSkipBootScreen = true,
                _ => romFileName = Some(arg.to_string())
            }
        }

        let ret = match romFileName {
            Some(rfn) => ProgramState::new(rfn, shouldSkipBootScreen),
            None => printUsageAndExit()
        };

        ret

    }
    else {
        printUsageAndExit()
    }

}


//TODO(DanB): Handle unwrap
fn main() {


    let mut prg = parseArgs();
    let mut gb = &mut *prg.gb;

    //load ROM
    let romData = match openROM(&prg.romFileName) {
        Ok(data) => data,
        Err(err) => panic!("{}", err)
    };


    gb.mem.romData = romData;

    //skip "Nintendo" logo if specified
    if prg.shouldSkipBootScreen {
        gb.cpu.PC = 0x100;
        gb.mem.inBios = false;
        gb.mem.lcd.mode = LCDMode::VBlank;
    }


    let mut cyclesPerDividerIncrement = 0u32;
    let mut cyclesPerTimerIncrement = 0u32;

    //init SDL 
    let sdlContext = sdl2::init().unwrap();

    let videoSubsystem = sdlContext.video().unwrap();
    let timer = sdlContext.timer().unwrap();
    let mut eventPump = sdlContext.event_pump().unwrap();

    let mut mainWindowHeight = WINDOW_HEIGHT;
    let mainWindowWidth = WINDOW_WIDTH;
    let mainWindow = videoSubsystem.window("GB Emu", mainWindowWidth, mainWindowHeight).position_centered().build().unwrap();
    let mainWindowID = mainWindow.id();
    let mut renderer = mainWindow.renderer().build().unwrap();


    //init debug screen
    let mut dbg = initDebug(0, WINDOW_HEIGHT as i32, WINDOW_WIDTH, WINDOW_HEIGHT / 2);

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
                                            WINDOW_HEIGHT + dbg.drawHeight
                                        }
                                        else {
                                            WINDOW_HEIGHT
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
                                        match dumpGameBoyState(gb, "dump.txt") {
                                            Ok(_) => { 
                                                println!("Wrote debug file");
                                            },
                                            Err(err) => {
                                                println!("{}", err);
                                            }
                                        }
                                    }
                                }

                                Keycode::Up => {
                                    gb.mem.joypad.up = ButtonState::Down;
                                    gb.mem.requestedInterrupts &= 1 << 4;
                                },
                                Keycode::Down => {
                                    gb.mem.joypad.down = ButtonState::Down;
                                    gb.mem.requestedInterrupts &= 1 << 4;
                                },
                                Keycode::Left => {
                                    gb.mem.joypad.left = ButtonState::Down;
                                    gb.mem.requestedInterrupts &= 1 << 4;
                                },
                                Keycode::Right => {
                                    gb.mem.joypad.right = ButtonState::Down;
                                    gb.mem.requestedInterrupts &= 1 << 4;
                                },

                                Keycode::X => {
                                    gb.mem.joypad.a = ButtonState::Down;
                                    gb.mem.requestedInterrupts &= 1 << 4;
                                },
                                Keycode::Z => {
                                    gb.mem.joypad.b = ButtonState::Down;
                                    gb.mem.requestedInterrupts &= 1 << 4;
                                },
                                Keycode::Return => {
                                    gb.mem.joypad.start = ButtonState::Down;
                                    gb.mem.requestedInterrupts &= 1 << 4;
                                }
                                Keycode::RShift => {
                                    gb.mem.joypad.select = ButtonState::Down;
                                    gb.mem.requestedInterrupts &= 1 << 4;
                                }

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
            while batchCycles < CYCLES_PER_SLEEP {

                    stepCPU(&mut gb.cpu, &mut gb.mem);

                    stepLCD(&mut gb.mem.lcd, &mut gb.mem.requestedInterrupts, gb.cpu.instructionCycles);
                    batchCycles += gb.cpu.instructionCycles;

                    cyclesPerDividerIncrement += gb.cpu.instructionCycles;

                    if cyclesPerDividerIncrement >= CYCLES_PER_DIVIDER_INCREMENT {
                        gb.mem.divider = gb.mem.divider.wrapping_add(1);
                        cyclesPerDividerIncrement -= CYCLES_PER_DIVIDER_INCREMENT;
                    }

                    //if timer is enabled...
                    //TODO: Timer may need to be more accurate in the case
                    //      when the timer is slowed down or sped up.  Cycles may need to be reset.
                    if gb.mem.isTimerEnabled {
                        cyclesPerTimerIncrement += gb.cpu.instructionCycles;


                        if cyclesPerTimerIncrement >= (gb.mem.timerMode as u32) {
                            gb.mem.timerCounter = gb.mem.timerCounter.wrapping_add(1);

                            if gb.mem.timerCounter == 0 {
                                gb.mem.timerCounter = gb.mem.timerModulo;
                                gb.mem.requestedInterrupts |= 1 << 2;
                                cyclesPerTimerIncrement -= gb.mem.timerMode as u32; 
                            }
                        }
                    }
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
