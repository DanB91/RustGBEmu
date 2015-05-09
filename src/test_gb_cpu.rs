/*
 * Will contain all of the unit tests for the CPU functions
 *
 */

use gb_cpu::*;
use gb_memory::*;
use gb_cpu::Flag::*;
static MBC0_ROM : &'static str = "samples/mbc0.gb";
fn tetrisMemoryState() -> MemoryState{
    let mut mem = MemoryState::new();

    let romData = match openROM(MBC0_ROM) {
        Ok(data) => data,
        Err(err) => panic!("{}", err)
    };
    mem.romData = romData;

    mem
}

fn testingCPU() -> CPUState {
    let mut cpu = CPUState::new();

    cpu.PC = 0xC000; //set PC to beginning of working RAM

    cpu
}

//NOTE(DanB): best for instructions that don't affect flags or require setup in memory
fn executeInstructionOnClearedState(instruction: u8) -> (CPUState, MemoryState) {
    let mut cpu = CPUState::new();
    let mut mem = tetrisMemoryState();

    let(newPC, cyclesTaken) = executeInstruction(instruction, &mut cpu, &mut mem);
    cpu.PC = newPC;
    cpu.instructionCycles = cyclesTaken;
    cpu.totalCycles += cyclesTaken;

    (cpu,mem)
}

#[test]
fn nop() { //0x0 

    let (cpu,_) = executeInstructionOnClearedState(0);

    assert!(cpu.instructionCycles == 4);
    assert!(cpu.PC == 1);

}

#[test]
fn loadImm16() { //0x1
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    mem.workingRAM[1] = 0xBB; //write AABB to memory location 1
    mem.workingRAM[2] = 0xAA;

    let (newPC, cyclesTaken) = executeInstruction(1, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 3);
    assert!(cyclesTaken == 12);

    assert!(word(cpu.B,cpu.C) == 0xAABB);
}

#[test]
fn loadAIntoMemory() { //0x2
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xCC;
    cpu.B = 0xC0;
    cpu.C = 0x00;
    //write 0xCC to beginning of working RAM (addr C000)
    let (newPC, cyclesTaken) = executeInstruction(2, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 8);

    assert!(mem.workingRAM[0] == 0xCC);

}

#[test]
fn increment16() { //0x3
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.B = 0x0C;
    cpu.C = 0xFF;

    //increment BC
    let (newPC, cyclesTaken) = executeInstruction(3, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 8);

    assert!(word(cpu.B,cpu.C) == 0xD00);
}

#[test]
fn increment8() { //0x4


    macro_rules! testInc8 {
        ($reg: ident, $instr: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();


            //test half carry and zero set
            cpu.$reg = 0xFF;

            let (newPC, cyclesTaken) = executeInstruction($instr, &mut cpu, &mut mem);

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 4);

            assert!(cpu.$reg == 0);

            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

            //test half carry and zero clear

            cpu = testingCPU();
            mem = tetrisMemoryState();
            cpu.$reg = 0x1;

            let (newPC, cyclesTaken) = executeInstruction($instr, &mut cpu, &mut mem);

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 4);

            assert!(cpu.$reg == 2);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));



        });
    }

    let instructions = [0x4u8, 0xC];

    for instruction in instructions.iter() {
        match *instruction {
            0x4 => testInc8!(B, 0x4),
            0xC => testInc8!(C, 0xC),
            _ => panic!("Unreachable")
        };
    }


}

#[test]
fn decrement8() { //0x5

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //test half carry set
    cpu.B = 0;

    let (newPC, cyclesTaken) = executeInstruction(5, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);

    assert!(cpu.B == 0xFF);

    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

    //test zero set

    cpu = testingCPU();
    mem = tetrisMemoryState();
    cpu.B = 0x1;

    let (newPC, cyclesTaken) = executeInstruction(5, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);

    assert!(cpu.B == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

}

#[test]
fn load8() {//0x6
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();
    let oldPC = cpu.PC;

    writeWordToMemory(&mut mem, 0xAA06, cpu.PC); //also loads instruction into memory

    step(&mut cpu, &mut mem);

    assert!(cpu.B == 0xAA);
    assert!(cpu.instructionCycles == 8);
    assert!(cpu.PC == oldPC + 2);
}

#[test]
fn rlca() { //0x7
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //test rotate 0
    let (newPC, cyclesTaken) = executeInstruction(0x7, &mut cpu, &mut mem);

    assert!(cpu.A == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);

    //test C set
    cpu.A = 0x88;


    let (newPC, cyclesTaken) = executeInstruction(0x7, &mut cpu, &mut mem);

    assert!(cpu.A == 0x11);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));


    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);

    //test C clear
    cpu.A = 0x7F;

    let (newPC, cyclesTaken) = executeInstruction(0x7, &mut cpu, &mut mem);

    assert!(cpu.A == 0xFE);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));


    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);
}

#[test]
fn loadSPIntoMemory() {//0x8
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.SP = 0xAAAA;
    writeWordToMemory(&mut mem, 0xDDDD, cpu.PC + 1);

    //test rotate 0
    let (newPC, cyclesTaken) = executeInstruction(0x8, &mut cpu, &mut mem);

    assert!(cyclesTaken == 20);
    assert!(newPC == cpu.PC + 3);
    assert!(readWordFromMemory(&mut mem, 0xDDDD) == 0xAAAA);

}

#[test]
fn addToHL() { //0x9
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //HL has 0x55AA
    cpu.H = 0x55;
    cpu.L = 0xAA;

    cpu.B = 0;
    cpu.C = 0x66;

    //55AA + 66 = 5610
    let (newPC, cyclesTaken) = executeInstruction(0x9, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(word(cpu.H, cpu.L) == 0x5610);
    assert!(word(cpu.B, cpu.C) == 0x66);

    //no flags set
    assert!(cpu.F == 0);

    //HL has 0xFFFF
    cpu.H = 0xFF;
    cpu.L = 0xFF;

    cpu.B = 0;
    cpu.C = 0x2;

    //FFFF + 2 = 1
    let (newPC, cyclesTaken) = executeInstruction(0x9, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(word(cpu.H, cpu.L) == 0x1);

    //H, C set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn loadFromMem8Bit() { //0xA

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    writeByteToMemory(&mut mem, 0xAA, 0xCCDD); //load AA to CCDD

    cpu.B = 0xCC;
    cpu.C = 0xDD;

    let (newPC, cyclesTaken) = executeInstruction(0xA, &mut cpu, &mut mem);


    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 8);
    assert!(cpu.A == 0xAA);

}

#[test]
fn decrement16() { //0xB
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.B = 0x00;
    cpu.C = 0x00;

    //increment BC
    let (newPC, cyclesTaken) = executeInstruction(0xB, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 8);

    assert!(word(cpu.B,cpu.C) == 0xFFFF);

}


