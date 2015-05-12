/*
 * Will contain all of the unit tests for the CPU functions
 *
 */


//NOTE(DanB): Testing excuteinstruction() instead of step() because step will be changed
//frequently, but excuteinstruction() should not be
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

//returns CPU with PC at 0xC000 which is the first address of RAM.  All addresses before 0xC000 are
//read only
fn testingCPU() -> CPUState {
    let mut cpu = CPUState::new();

    cpu.PC = 0xC000; //set PC to beginning of working RAM

    cpu
}


/*
 * Use this macro to run test functions associated with certian CPU instructions.
 *
 * Args:
 *      inst: The opcode of the instruction.  e.g. 0x2
 *      testFn: The test function to be run with that opcode
 *
 *      These inst, testFn pairs are separated by ';'
 *
 *
 * Examples:
 *      test!(0x2, testLoadRegIntoMem!(A, B, C, 0x2);
 *           0x12, testLoadRegIntoMem!(A, D, E, 0x12));
 *  
 */
macro_rules! test {
    ($($inst: expr, $testFn: expr); *) => ({

        let insts = [$($inst), *];

        for inst in &insts {
            println!("Testing: {:X}", *inst); 
            
            match *inst {
                $($inst => $testFn), *,
                _ => panic!("Unreachable")
            };
        }
    })
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


    macro_rules! testLoadImm16 {
        ($high: ident, $low: ident, $inst: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            mem.workingRAM[1] = 0xBB; //write AABB to memory location 1
            mem.workingRAM[2] = 0xAA;

            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(newPC == cpu.PC + 3);
            assert!(cyclesTaken == 12);

            assert!(word(cpu.$high,cpu.$low) == 0xAABB);
        })
    }

    let insts = [0x1, 0x11, 0x21];

    for inst in &insts {
        match *inst {
            0x1 => testLoadImm16!(B, C, 1),
            0x11 => testLoadImm16!(D, E, 0x11),
            0x21 => testLoadImm16!(H, L, 0x21),
            _ => panic!("Unreachable")
        }
    }

}

#[test]
fn loadRegIntoMemory() { //0x2, 0x12

    //used for HL+, HL-
    enum OpOnHL {
        Add,
        Sub
    }

    macro_rules! testLoadRegIntoMem {
        ($destReg: ident, $addrHigh: ident, $addrLow: ident, $inst: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.$destReg = 0xCC;
            cpu.$addrHigh = 0xC0;
            cpu.$addrLow = 0x00;
            //write 0xCC to beginning of working RAM (addr C000)
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);

            assert!(mem.workingRAM[0] == 0xCC);
        });

        ($destReg: ident, $addrHigh: ident, $addrLow: ident, $opOnHL: expr, $inst: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.$destReg = 0xCC;
            cpu.$addrHigh = 0xC0;
            cpu.$addrLow = 0x00;
            //write 0xCC to beginning of working RAM (addr C000)
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);


            match $opOnHL {
                OpOnHL::Add => assert!(word(cpu.$addrHigh, cpu.$addrLow) == 0xC001),
                OpOnHL::Sub => assert!(word(cpu.$addrHigh, cpu.$addrLow) == 0xEFFF)
            };

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);

            assert!(mem.workingRAM[0] == 0xCC);
        })
    }


    test!(0x2, testLoadRegIntoMem!(A, B, C, 0x2);
          0x12, testLoadRegIntoMem!(A, D, E, 0x12);
          0x22, testLoadRegIntoMem!(A, H, L, OpOnHL::Add, 0x22)
          );

        
    

}

#[test]
fn increment16() { //0x3, 0x13
    macro_rules! testIncrement16 {
        ($highReg: ident, $lowReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.$highReg = 0x0C;
            cpu.$lowReg = 0xFF;

            //increment BC
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);

            assert!(word(cpu.$highReg,cpu.$lowReg) == 0xD00);

        })
    }

    test!(3, testIncrement16!(B,C, 3);
          0x13, testIncrement16!(D,E, 0x13));
    
}

#[test]
fn increment8() { //0x4, 0xC, 0x14, 0x1C


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

    test!(0x4, testInc8!(B, 0x4);
          0xC, testInc8!(C, 0xC);
          0x14, testInc8!(D, 0x14);
          0x1C, testInc8!(E, 0x1C)
          );

}

#[test]
fn decrement8() { //0x5, 0xD, 0x15

    macro_rules! testDec8 {

        ($reg: ident, $instr: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            //test half carry set
            cpu.$reg = 0;

            let (newPC, cyclesTaken) = executeInstruction($instr, &mut cpu, &mut mem);

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 4);

            assert!(cpu.$reg == 0xFF);

            assert!(isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            //test zero set

            cpu = testingCPU();
            mem = tetrisMemoryState();
            cpu.$reg = 0x1;

            let (newPC, cyclesTaken) = executeInstruction($instr, &mut cpu, &mut mem);

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 4);

            assert!(cpu.$reg == 0);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));
        });
    }
    
    test!(0x5, testDec8!(B, 0x5);
          0xD, testDec8!(C, 0xD);
          0x15, testDec8!(D, 0x15);
          0x1D, testDec8!(E, 0x1D)
          );


}

#[test]
fn load8() {//0x6, 0xE, 0x16, 0x1E
    macro_rules! testLoad8 {
        ($reg: ident, $instr: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();
            let oldPC = cpu.PC;

            writeWordToMemory(&mut mem, $instr, cpu.PC); //load instruction
            writeWordToMemory(&mut mem, 0xAA, cpu.PC+1); //load value

            step(&mut cpu, &mut mem);

            assert!(cpu.$reg == 0xAA);
            assert!(cpu.instructionCycles == 8);
            assert!(cpu.PC == oldPC + 2);
        })
    }

    test!(0x6, testLoad8!(B, 6);
          0xE, testLoad8!(C, 0xE);
          0x16, testLoad8!(D, 0x16);
          0x1E, testLoad8!(E, 0x1E)
         );
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
fn addToHL() { //0x9, 0x19
    macro_rules! testAddToHL {
        ($highReg: ident, $lowReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            //HL has 0x55AA
            cpu.H = 0x55;
            cpu.L = 0xAA;

            cpu.$highReg = 0;
            cpu.$lowReg = 0x66;

            //55AA + 66 = 5610
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 1);
            assert!(word(cpu.H, cpu.L) == 0x5610);
            assert!(word(cpu.$highReg, cpu.$lowReg) == 0x66);

            //no flags set
            assert!(cpu.F == 0);

            //HL has 0xFFFF
            cpu.H = 0xFF;
            cpu.L = 0xFF;

            cpu.$highReg = 0;
            cpu.$lowReg = 0x2;

            //FFFF + 2 = 1
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 1);
            assert!(word(cpu.H, cpu.L) == 0x1);

            //H, C set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

        })
    }

    test!(0x9, testAddToHL!(B,C, 0x9);
          0x19, testAddToHL!(D,E, 0x19)
          );
    

}

#[test]
fn loadFromMem8Bit() { //0xA

    macro_rules! testLoadFromMem8 {
        ($destReg: ident, $highAddr: ident, $lowAddr: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            writeByteToMemory(&mut mem, 0xAA, 0xCCDD); //load AA to CCDD

            cpu.$highAddr = 0xCC;
            cpu.$lowAddr = 0xDD;

            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);


            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);
            assert!(cpu.$destReg == 0xAA);

        })
    }

    test!(0xA, testLoadFromMem8!(A, B, C, 0xA);
          0x1A, testLoadFromMem8!(A, D, E, 0x1A)
          );

}

#[test]
fn decrement16() { //0xB, 0x1B

    macro_rules! testDecrement16 {
        ($highReg: ident, $lowReg: ident, $inst: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.$highReg = 0x00;
            cpu.$lowReg = 0x00;

            //increment BC
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);

            assert!(word(cpu.$highReg,cpu.$lowReg) == 0xFFFF);
        })
    }


    test!(0xB, testDecrement16!(B,C, 0xB);
          0x1B, testDecrement16!(D,E, 0x1B));

}

#[test]
fn rotateRight() { //0xF

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //test rotate 0
    let (newPC, cyclesTaken) = executeInstruction(0xF, &mut cpu, &mut mem);

    assert!(cpu.A == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);

    //test C set
    cpu.A = 0x11;


    let (newPC, cyclesTaken) = executeInstruction(0xF, &mut cpu, &mut mem);

    assert!(cpu.A == 0x88);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));


    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);

    //test C clear
    cpu.A = 0x76;

    let (newPC, cyclesTaken) = executeInstruction(0xF, &mut cpu, &mut mem);

    assert!(cpu.A == 0x3B);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));


    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);
}



#[test]
fn rotateLeftThroughCarry() { //0x17


    macro_rules! testRLA {
        ($regAVal: expr, $expectedVal: expr, $setC: expr, $isCSet: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = $regAVal;

            if $setC {
                setFlag(Carry, &mut cpu.F);
            }

            //test rotate 0
            let (newPC, cyclesTaken) = executeInstruction(0x17, &mut cpu, &mut mem);

            assert!(cpu.A == $expectedVal);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

            if $isCSet {
                assert!(isFlagSet(Carry, cpu.F));
            }
            else {
                assert!(!isFlagSet(Carry, cpu.F));
            }

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 4);


        })
    }


    //test rotate 0
    testRLA!(0, 0, false, false);

    //test C will be set
    testRLA!(0x88, 0x10, false, true);

    //test C clear
    testRLA!(0x7F, 0xFE, false, false);

    //test with C already set
    testRLA!(0x80, 0x1, true, true);
    
}

#[test]
fn jumpRelative() { //0x18

    //NOTE(DanB): This mostly tests that the signed conversion works
    fn testJR(uOffset: u8, sOffset: i8) {
        let mut cpu = testingCPU();
        let mut mem = tetrisMemoryState();


        //load offset 
        writeByteToMemory(&mut mem, uOffset, cpu.PC+1);

        let (newPC, cyclesTaken) = executeInstruction(0x18, &mut cpu, &mut mem);

        assert!(newPC  == (cpu.PC as i16 + sOffset as i16 + 2) as u16);
        assert!(cyclesTaken == 12);

    }

    testJR(0x80, -128);
    testJR(0x7F, 127);
}

#[test]
fn jumpRelativeWithCondition() { //0x20

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //test Z flag cleared.  should perform jump
    clearFlag(Zero, &mut cpu.F);

    //load offset 
    writeByteToMemory(&mut mem, 0x80, cpu.PC+1);

    let (newPC, cyclesTaken) = executeInstruction(0x20, &mut cpu, &mut mem);

    assert!(newPC  == (cpu.PC as i16 - 128 + 2) as u16);
    assert!(cyclesTaken == 12);
    
    //test Z flag set.  should not perform jump
    setFlag(Zero, &mut cpu.F);

    //load offset 
    writeByteToMemory(&mut mem, 0x80, cpu.PC+1);

    let (newPC, cyclesTaken) = executeInstruction(0x20, &mut cpu, &mut mem);

    assert!(newPC  == (cpu.PC as i16 + 2) as u16);
    assert!(cyclesTaken == 8);
}

#[test]
fn rotateRightThroughCarry() { //0x1F


    macro_rules! testRRA {
        ($regAVal: expr, $expectedVal: expr, $setC: expr, $isCSet: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = $regAVal;

            if $setC {
                setFlag(Carry, &mut cpu.F);
            }

            //test rotate 0
            let (newPC, cyclesTaken) = executeInstruction(0x1F, &mut cpu, &mut mem);

            assert!(cpu.A == $expectedVal);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

            if $isCSet {
                assert!(isFlagSet(Carry, cpu.F));
            }
            else {
                assert!(!isFlagSet(Carry, cpu.F));
            }

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 4);


        })
    }


    //test rotate 0
    testRRA!(0, 0, false, false);

    //test C will be set
    testRRA!(0x11, 0x8, false, true);

    //test C clear
    testRRA!(0x88, 0x44, false, false);

    //test with C already set
    testRRA!(0x81, 0xC0, true, true);
}
