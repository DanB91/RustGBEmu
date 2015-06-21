#![allow(non_snake_case)]
#![allow(dead_code)]

use std::env;
use std::io;

mod gb_memory;
use gb_memory::*;

mod gb_cpu;
use gb_cpu::*;


static USAGE: &'static str= "Usage: gbemu path_to_rom";

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

fn disassemble(cpu: &CPUState, mem: &MemoryState) -> String {
    let instruction = readByteFromMemory(mem, cpu.PC);

    macro_rules! nextByte {
        () => (readByteFromMemory(mem, cpu.PC.wrapping_add(1)))
    }

    macro_rules! nextWord {
        () => (readWordFromMemory(mem, cpu.PC.wrapping_add(1)))
    }
    match instruction {
        0 => format!("NOP"),
        1 => format!("LD BC ${:X}", nextWord!()),
        2 => format!("LD (BC) A"),
        3 => format!("INC BC"),
        4 => format!("INC B"),
        5 => format!("DEC B"),
        6 => format!("LD B ${:X}", nextByte!()),

        0x20 => format!("JR NZ {}", nextByte!() as i8),
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

}


