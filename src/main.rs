#![allow(non_snake_case)]
#![allow(dead_code)]

use std::env;
use std::io;

mod gb_memory;
use gb_memory::*;

mod gb_cpu;
use gb_cpu::*;

#[macro_use]
extern crate bitflags;

extern crate sdl2;
extern crate libc;
extern crate sdl2_sys;
mod sdl2_ttf;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::timer::{get_performance_counter, get_performance_frequency};

use std::path::Path;

static USAGE: &'static str= "Usage: gbemu path_to_rom";
static FONT_PATH_STR: &'static str = "res/Gamegirl.ttf";

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

// fail when error
macro_rules! trying(
    ($e:expr) => (match $e { Ok(e) => e, Err(e) => panic!("failed: {}", e) })
);

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



fn main() {

    //parse cmd args
    let fileName = match getROMFileName() {
        Ok(fileName) => fileName,
        Err(err) => {
            println!("{}", err);
            return
        }
    };

    let mut cpu = CPUState::new();

    //load ROM
    let romData = match openROM(&fileName[..]) {
        Ok(data) => data,
        Err(err) => panic!("{}", err)
    };

    let mut mem = MemoryState::new();
    mem.romData = romData;


    //init SDL 
    let mut sdlContext = sdl2::init().video().events().unwrap();
    sdl2_ttf::init().unwrap();

    let window = sdlContext.window("GB Emu", SCREEN_WIDTH, SCREEN_HEIGHT).position_centered().build().unwrap();
    let mut renderer = window.renderer().build().unwrap();
    let font =  trying!(sdl2_ttf::Font::from_file(Path::new(FONT_PATH_STR), 14));

    


    'gameBoyLoop: loop {
        let start = get_performance_counter();
        let mut framesPerSec = 0f32;

        for event in sdlContext.event_pump().poll_iter() {

            match event {
                Event::Quit{..} => break 'gameBoyLoop,
                _ => {}
            }   
        }


        let mut instructionToPrint = readByteFromMemory(&mem, cpu.PC) as u16;

        if instructionToPrint == 0xCB {
            instructionToPrint =  word(0xCBu8, readByteFromMemory(&mem, cpu.PC.wrapping_add(1)))
        }

        let toPrint = format!("{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",  
                              format!("Current Insruction: {}\tOpcode:{:X}", disassemble(&cpu, &mem), instructionToPrint),
                              format!("Total Cycles: {}, Cycles just executed: {}", cpu.totalCycles, cpu.instructionCycles),
                              format!("Frames per second {}", framesPerSec),
                              format!("Currently in BIOS: {}", mem.inBios),
                              format!("Flags: Z: {}, N: {}, H: {}, C: {}", isFlagSet(Flag::Zero, cpu.F), isFlagSet(Flag::Neg, cpu.F), isFlagSet(Flag::Half, cpu.F), isFlagSet(Flag::Carry, cpu.F)),
                              format!("PC: {:X}\tSP: {:X}", cpu.PC, cpu.SP),
                              format!("A: {:X}\tF: {:X}\tB: {:X}\tC: {:X}", cpu.A, cpu.F, cpu.B, cpu.C),
                              format!("D: {:X}\tE: {:X}\tH: {:X}\tL: {:X}", cpu.D, cpu.E, cpu.H, cpu.L));


        let fontSurf =  trying!(font.render_str_blended_wrapped(&toPrint, Color::RGBA(255,255,255,255), SCREEN_WIDTH));
        let mut fontTex = trying!(renderer.create_texture_from_surface(&fontSurf));

        let (texW, texH) = { let q = fontTex.query(); (q.width, q.height)};
        renderer.clear();
        renderer.copy(&mut fontTex, None, sdl2::rect::Rect::new(0, texH as i32, texW, texH).unwrap());
        renderer.present();

        step(&mut cpu, &mut mem);

        framesPerSec = 1f32 / ((get_performance_counter() as f64 - start as f64) / get_performance_frequency() as f64) as f32;



    }

    sdl2_ttf::quit();

    //TODO(DanB): port this printing over to drawing to SDL screen
    /*
    let mut stdin = io::stdin();
    let mut line = String::new();

    
    //step one instruction every time you hit enter
    loop {
        let mut instructionToPrint = readByteFromMemory(&mem, cpu.PC) as u16;

        if instructionToPrint == 0xCB {
            instructionToPrint =  word(0xCBu8, readByteFromMemory(&mem, cpu.PC.wrapping_add(1)))
        }

        println!("Current Insruction: {}\tOpcode:{:X}", disassemble(&cpu, &mem), instructionToPrint);
        println!("Total Cycles: {}, Cycles just executed: {}", cpu.totalCycles, cpu.instructionCycles);
        println!("Currently in BIOS: {}", mem.inBios);
        println!("Flags: Z: {}, N: {}, H: {}, C: {}", isFlagSet(Flag::Zero, cpu.F), isFlagSet(Flag::Neg, cpu.F), isFlagSet(Flag::Half, cpu.F), isFlagSet(Flag::Carry, cpu.F));
        println!("PC: {:X}\tSP: {:X}", cpu.PC, cpu.SP);
        println!("A: {:X}\tF: {:X}\tB: {:X}\tC: {:X}", cpu.A, cpu.F, cpu.B, cpu.C);
        println!("D: {:X}\tE: {:X}\tH: {:X}\tL: {:X}", cpu.D, cpu.E, cpu.H, cpu.L);

        match stdin.read_line(&mut line) {
            Ok(_) => {step(&mut cpu, &mut mem);}
            Err(_) => {
                break;
            }
        }


    }
    */

}



