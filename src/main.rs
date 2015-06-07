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


fn main() {

    //parse cmd args
    let fileName = match getROMFileName() {
        Ok(fileName) => fileName,
        Err(err) => {
            println!("{}", err);
            return
        }
    };

    let mut totalCycles = 0u32; //total cycles since game has been loaded
    let mut instructionCycles = 0u32; //number of cycles in a given instruction

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

        match stdin.read_line(&mut line) {
            Ok(_) => {}
            Err(_) => {
                break;
            }
        }
        let instructionToExecute = readByteFromMemory(&mem, cpu.PC);
        println!("Current Insruction: {:X}", instructionToExecute);
        println!("Total Cycles: {}, Cycles just executed: {}", totalCycles, instructionCycles);
        println!("Currently in BIOS: {}", mem.inBios);
        println!("Flags: Z: {}, N: {}, H: {}, C: {}", isFlagSet(Flag::Zero, cpu.F), isFlagSet(Flag::Neg, cpu.F), isFlagSet(Flag::Half, cpu.F), isFlagSet(Flag::Carry, cpu.F));
        println!("PC: {:X}\tSP: {:X}", cpu.PC, cpu.SP);
        println!("A: {:X}\tF: {:X}\tB: {:X}\tC: {:X}", cpu.A, cpu.F, cpu.B, cpu.C);
        println!("D: {:X}\tE: {:X}\tH: {:X}\tL: {:X}", cpu.D, cpu.E, cpu.H, cpu.L);


        if cpu.PC > 0xFF {
            mem.inBios = false;
        }

        let (newPC, cyclesTaken) = executeInstruction(instructionToExecute, &mut cpu, &mut mem); 
        cpu.PC = newPC;
        instructionCycles = cyclesTaken;

        totalCycles += instructionCycles;

    }

}






