/*
 * Will contain all of the unit tests for the CPU functions
 *
 */


//NOTE(DanB): Testing excuteinstruction() instead of step() because step will be changed
//frequently, but excuteinstruction() should not be.
//
//Another NOTE:  I mix and match between using internal functions and internal macros when testing
//more than one function that uses similar code.  I have come to the conclusion that macros seem to
//be a bit better since "cargo test" will actuall print out the calling macro.  Also, I have
//started to use assert_eq instead of assert since assert_eq will print out both sides of the
//equation.
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
    cpu.SP = 0xFFFE; //set stack to start at FFFE

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
                OpOnHL::Sub => assert!(word(cpu.$addrHigh, cpu.$addrLow) == 0xBFFF)
            };

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);

            assert!(mem.workingRAM[0] == 0xCC);
        })
    }


    test!(0x2, testLoadRegIntoMem!(A, B, C, 0x2);
          0x12, testLoadRegIntoMem!(A, D, E, 0x12);
          0x22, testLoadRegIntoMem!(A, H, L, OpOnHL::Add, 0x22);
          0x32, testLoadRegIntoMem!(A, H, L, OpOnHL::Sub, 0x32)
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
          0x13, testIncrement16!(D,E, 0x13);
          0x23, testIncrement16!(H,L,0x23));

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
          0x1C, testInc8!(E, 0x1C);
          0x24, testInc8!(H, 0x24);
          0x2C, testInc8!(L, 0x2C);
          0x3C, testInc8!(A, 0x3C)
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
          0x1D, testDec8!(E, 0x1D);
          0x25, testDec8!(H, 0x25);
          0x2D, testDec8!(L, 0x2D);
          0x3D, testDec8!(A, 0x3D)
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
          0x1E, testLoad8!(E, 0x1E);
          0x26, testLoad8!(H, 0x26);
          0x2E, testLoad8!(L, 0x2E);
          0x3E, testLoad8!(A, 0x3E)
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
    assert!(isFlagSet(Zero, cpu.F));
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
fn addHLToHL() { //0x29
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //HL has 0x1077
    cpu.H = 0x10;
    cpu.L = 0x77;


    //1077 * 2 = 20EE
    let (newPC, cyclesTaken) = executeInstruction(0x29, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(word(cpu.H, cpu.L) == 0x20EE);

    //no flags set
    assert!(cpu.F == 0);

    //HL has 0xFFFF
    cpu.H = 0xFF;
    cpu.L = 0xFF;

    //FFFF * 2 = 0x1FFFE
    let (newPC, cyclesTaken) = executeInstruction(0x29, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(word(cpu.H, cpu.L) == 0xFFFE);

    //H, C set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));



}

#[test]
fn addSPToHL() { //0x39
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //HL has 0x1077
    cpu.H = 0x10;
    cpu.L = 0x77;

    cpu.SP = 0x1122;

    //0x1077 + 0x1122 = 2199
    let (newPC, cyclesTaken) = executeInstruction(0x39, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(word(cpu.H, cpu.L) == 0x2199);

    //no flags set
    assert!(cpu.F == 0);

    //HL has 0xFFFF
    cpu.H = 0xFF;
    cpu.L = 0xFF;

    cpu.SP = 0x2;

    //FFFF + 2 = 1
    let (newPC, cyclesTaken) = executeInstruction(0x39, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(word(cpu.H, cpu.L) == 1);

    //H, C set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn loadFromMem8Bit() { //0xA

    //used for HL+, HL-
    enum OpOnHL {
        Add,
        Sub
    }

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

        });

        ($destReg: ident, $highAddr: ident, $lowAddr: ident, $opOnHL: expr, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            writeByteToMemory(&mut mem, 0xAA, 0xCCDD); //load AA to CCDD

            cpu.$highAddr = 0xCC;
            cpu.$lowAddr = 0xDD;

            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            match $opOnHL {
                OpOnHL::Add => assert!(word(cpu.H,cpu.L) == 0xCCDE),
                OpOnHL::Sub => assert!(word(cpu.H,cpu.L) == 0xCCDC),

            };

            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);
            assert!(cpu.$destReg == 0xAA);

        })
    }

    test!(0xA, testLoadFromMem8!(A, B, C, 0xA);
          0x1A, testLoadFromMem8!(A, D, E, 0x1A);
          0x2A, testLoadFromMem8!(A, H, L, OpOnHL::Add, 0x2A)
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
          0x1B, testDecrement16!(D,E, 0x1B);
          0x2B, testDecrement16!(H,L, 0x2B)
         );

}

#[test]
fn rotateRight() { //0xF

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //test rotate 0
    let (newPC, cyclesTaken) = executeInstruction(0xF, &mut cpu, &mut mem);

    assert!(cpu.A == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
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
            
            if $expectedVal == 0 {
                assert!(isFlagSet(Zero, cpu.F));
            }
            else {
                assert!(!isFlagSet(Zero, cpu.F));
            }

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

        assert!(newPC  == (cpu.PC as i16 + sOffset as i16 + 2 ) as u16);
        assert!(cyclesTaken == 12);

    }

    testJR(0x80, -128);
    testJR(0x7F, 127);
}

#[test]
fn jumpRelativeWithCondition() { //0x20

    fn testJRC(flag: Flag, shouldBeSet: bool, inst: u8) {
        let mut cpu = testingCPU();
        let mut mem = tetrisMemoryState();

        //should perform jump
        if shouldBeSet {
            setFlag(flag, &mut cpu.F);
        }
        else {
            clearFlag(flag, &mut cpu.F);
        }

        //load offset 
        writeByteToMemory(&mut mem, 0x80, cpu.PC+1);

        let (newPC, cyclesTaken) = executeInstruction(inst, &mut cpu, &mut mem);

        assert!(newPC  == (cpu.PC as i16 - 128 )  as u16 + 2);
        assert!(cyclesTaken == 12);

        //should not perform jump
        if !shouldBeSet {
            setFlag(flag, &mut cpu.F);
        }
        else {
            clearFlag(flag, &mut cpu.F);
        }

        //load offset 
        writeByteToMemory(&mut mem, 0x80, cpu.PC+1);

        let (newPC, cyclesTaken) = executeInstruction(inst, &mut cpu, &mut mem);

        assert!(newPC  == (cpu.PC as i16 + 2) as u16);
        assert!(cyclesTaken == 8);

    }
    testJRC(Zero, false, 0x20);
    testJRC(Zero, true, 0x28);
    testJRC(Carry, false, 0x30);
    testJRC(Carry, true, 0x38);

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

            if $expectedVal == 0 {
                assert!(isFlagSet(Zero, cpu.F));
            }
            else {
                assert!(!isFlagSet(Zero, cpu.F));
            }
            
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

fn decimalAdjust() { //0x27
    //TODO: Cannot be implemented until 8-bit add and subtract are finished.
}

#[test]
fn complementA() { //0x2F

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    let (newPC, cyclesTaken) = executeInstruction(0x2F, &mut cpu, &mut mem);


    //1s complement of 0xAA is 0x55
    assert!(cpu.A == 0x55);

    assert!(isFlagSet(Neg, cpu.F));
    assert!(isFlagSet(Half, cpu.F));

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);
}

#[test]
fn loadImm16IntoSP() { //0x31

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    mem.workingRAM[1] = 0xBB; //write AABB to memory location 1
    mem.workingRAM[2] = 0xAA;

    let (newPC, cyclesTaken) = executeInstruction(0x31, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 3);
    assert!(cyclesTaken == 12);

    assert!(cpu.SP == 0xAABB);

}

#[test]
fn incrementSP() { //0x33

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.SP = 0xCFFF;

    //increment SP
    let (newPC, cyclesTaken) = executeInstruction(0x33, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 8);

    assert!(cpu.SP == 0xD000);
}

#[test]
fn decrementSP() { //0x3B

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.SP = 0xD000;

    //decrement SP
    let (newPC, cyclesTaken) = executeInstruction(0x3B, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 8);

    assert!(cpu.SP == 0xCFFF);
}

#[test]
fn incrementValAtHL() { //0x34
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //increment value at 0xCCDD
    cpu.H = 0xCC;
    cpu.L = 0xDD;

    //test half carry and zero set
    writeByteToMemory(&mut mem, 0xFF, 0xCCDD);

    let (newPC, cyclesTaken) = executeInstruction(0x34, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 12);

    assert!(readByteFromMemory(&mem, 0xCCDD) == 0);

    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    //test half carry and zero clear

    writeByteToMemory(&mut mem, 0x1, 0xCCDD);

    let (newPC, cyclesTaken) = executeInstruction(0x34, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 12);

    assert!(readByteFromMemory(&mem, 0xCCDD) == 2);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));


}

#[test]
fn decrementValAtHL() { //0x35
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //decrement value at 0xCCDD
    cpu.H = 0xCC;
    cpu.L = 0xDD;

    //test half carry
    writeByteToMemory(&mut mem, 0, 0xCCDD);

    let (newPC, cyclesTaken) = executeInstruction(0x35, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 12);

    assert!(readByteFromMemory(&mem, 0xCCDD) == 0xFF);

    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

    //test zero set

    writeByteToMemory(&mut mem, 0x1, 0xCCDD);

    let (newPC, cyclesTaken) = executeInstruction(0x35, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 12);

    assert!(readByteFromMemory(&mem, 0xCCDD) == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

    //test nothing set

    writeByteToMemory(&mut mem, 0xFF, 0xCCDD);

    let (newPC, cyclesTaken) = executeInstruction(0x35, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 12);

    assert!(readByteFromMemory(&mem, 0xCCDD) == 0xFE);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

}


#[test]
fn loadImm8ToMemPointedAtHL() { //0x36

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    //decrement value at 0xCCDD
    cpu.H = 0xCC;
    cpu.L = 0xDD;

    //test half carry
    writeByteToMemory(&mut mem, 0xAA, cpu.PC + 1);

    let (newPC, cyclesTaken) = executeInstruction(0x36, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 2);
    assert!(cyclesTaken == 12);

    assert!(readByteFromMemory(&mem, 0xCCDD) == 0xAA);
}

#[test]
fn setCarry() { //0x37
    let (cpu, _) = executeInstructionOnClearedState(0x37);

    assert!(cpu.PC == 1);
    assert!(cpu.instructionCycles == 4);
    assert!(isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn complementCarry() { //0x3F
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.F = 0xF0;  //set all flags

    let (newPC, cyclesTaken) = executeInstruction(0x3F, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);

    //everything but Zero flag should be off since Zero is not affected
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Half, cpu.F));

    cpu.F = 0x0;  //clear all flags

    let (newPC, cyclesTaken) = executeInstruction(0x3F, &mut cpu, &mut mem);

    assert!(newPC == cpu.PC + 1);
    assert!(cyclesTaken == 4);

    //everything but Carry flag should be off 
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Half, cpu.F));
}

#[test]
fn load8BitReg() { //0x40 - 0x7F

    //used for register to register loading
    macro_rules! load8BitRegFromReg {
        ($dest: ident, $src: ident, $instr: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.$src = 0xAA;

            let (newPC, cyclesTaken) = executeInstruction($instr, &mut cpu, &mut mem);

            assert!(cpu.$dest == 0xAA);
            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 4);
        });


    }

    //used for (HL) to register loading
    macro_rules! load8BitRegFromMem {

        ($dest: ident, $instr: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.H = 0xCC;
            cpu.L = 0xDD;

            writeByteToMemory(&mut mem, 0xAA, 0xCCDD);

            let (newPC, cyclesTaken) = executeInstruction($instr, &mut cpu, &mut mem);

            assert!(cpu.$dest == 0xAA);
            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);
        });
    }


    //used for (HL) to register loading
    macro_rules! load8BitMemFromReg {

        ($src: ident, $instr: expr) => ({

            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.H = 0xCC;
            cpu.L = 0xCC;

            cpu.$src = 0xCC;

            let (newPC, cyclesTaken) = executeInstruction($instr, &mut cpu, &mut mem);

            assert!(readByteFromMemory(&mut mem, 0xCCCC) == 0xCC);
            assert!(newPC == cpu.PC + 1);
            assert!(cyclesTaken == 8);
        });
    }

    load8BitRegFromReg!(B, B, 0x40);
    load8BitRegFromReg!(B, C, 0x41);
    load8BitRegFromReg!(B, D, 0x42);
    load8BitRegFromReg!(B, E, 0x43);
    load8BitRegFromReg!(B, H, 0x44);
    load8BitRegFromReg!(B, L, 0x45);
    load8BitRegFromMem!(B, 0x46);
    load8BitRegFromReg!(B, A, 0x47);

    load8BitRegFromReg!(C, B, 0x48);
    load8BitRegFromReg!(C, C, 0x49);
    load8BitRegFromReg!(C, D, 0x4A);
    load8BitRegFromReg!(C, E, 0x4B);
    load8BitRegFromReg!(C, H, 0x4C);
    load8BitRegFromReg!(C, L, 0x4D);
    load8BitRegFromMem!(C, 0x4E);
    load8BitRegFromReg!(C, A, 0x4F);

    load8BitRegFromReg!(D, B, 0x50);
    load8BitRegFromReg!(D, C, 0x51);
    load8BitRegFromReg!(D, D, 0x52);
    load8BitRegFromReg!(D, E, 0x53);
    load8BitRegFromReg!(D, H, 0x54);
    load8BitRegFromReg!(D, L, 0x55);
    load8BitRegFromMem!(D, 0x56);
    load8BitRegFromReg!(D, A, 0x57);

    load8BitRegFromReg!(E, B, 0x58);
    load8BitRegFromReg!(E, C, 0x59);
    load8BitRegFromReg!(E, D, 0x5A);
    load8BitRegFromReg!(E, E, 0x5B);
    load8BitRegFromReg!(E, H, 0x5C);
    load8BitRegFromReg!(E, L, 0x5D);
    load8BitRegFromMem!(E, 0x5E);
    load8BitRegFromReg!(E, A, 0x5F);

    load8BitRegFromReg!(H, B, 0x60);
    load8BitRegFromReg!(H, C, 0x61);
    load8BitRegFromReg!(H, D, 0x62);
    load8BitRegFromReg!(H, E, 0x63);
    load8BitRegFromReg!(H, H, 0x64);
    load8BitRegFromReg!(H, L, 0x65);
    load8BitRegFromMem!(H, 0x66);
    load8BitRegFromReg!(H, A, 0x67);

    load8BitRegFromReg!(L, B, 0x68);
    load8BitRegFromReg!(L, C, 0x69);
    load8BitRegFromReg!(L, D, 0x6A);
    load8BitRegFromReg!(L, E, 0x6B);
    load8BitRegFromReg!(L, H, 0x6C);
    load8BitRegFromReg!(L, L, 0x6D);
    load8BitRegFromMem!(L, 0x6E);
    load8BitRegFromReg!(L, A, 0x6F);

    load8BitMemFromReg!(B, 0x70);
    load8BitMemFromReg!(C, 0x71);
    load8BitMemFromReg!(D, 0x72);
    load8BitMemFromReg!(E, 0x73);
    load8BitMemFromReg!(H, 0x74);
    load8BitMemFromReg!(L, 0x75);
    load8BitMemFromReg!(A, 0x77);

    load8BitRegFromReg!(A, B, 0x78);
    load8BitRegFromReg!(A, C, 0x79);
    load8BitRegFromReg!(A, D, 0x7A);
    load8BitRegFromReg!(A, E, 0x7B);
    load8BitRegFromReg!(A, H, 0x7C);
    load8BitRegFromReg!(A, L, 0x7D);
    load8BitRegFromMem!(A, 0x7E);
    load8BitRegFromReg!(A, A, 0x7F);
}

#[test]
fn halt() { //0x76
    //TODO(DanB): to be tested properly implemeted....


    let (cpu, _) = executeInstructionOnClearedState(0x76);

    assert!(cpu.PC == 1);
    assert!(cpu.instructionCycles == 4);
}

#[test]
fn add8Bit() { //0x80-0x85

    macro_rules! testAdd8 {
        //add registers
        ($srcReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            cpu.$srcReg = 0x11;

            //AA + 11 = BB
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xBB);
            assert!(cpu.$srcReg == 0x11);

            //no flags set
            assert!(cpu.F == 0);


            cpu.A = 0xAE;

            cpu.$srcReg = 2;

            //AE + 2 = B0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xB0);

            //H set
            assert!(isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

            cpu.A = 0xFF;

            cpu.$srcReg = 1;

            //FF + 1 = 0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x0);

            //C, Z, H set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

        });


    }


    testAdd8!(B, 0x80);
    testAdd8!(C, 0x81);
    testAdd8!(D, 0x82);
    testAdd8!(E, 0x83);
    testAdd8!(H, 0x84);
    testAdd8!(L, 0x85);
}

#[test]
fn add8BitFromMemAtHL() { //0x86

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    //HL has CCDD 
    cpu.H = 0xCC;
    cpu.L = 0xDD;


    //memory has 0x11
    writeByteToMemory(&mut mem, 0x11, 0xCCDD);

    //AA + 11 = BB
    let (newPC, cyclesTaken) = executeInstruction(0x86, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xBB);
    assert!(readByteFromMemory(&mem, 0xCCDD) == 0x11);

    //no flags set
    assert!(cpu.F == 0);


    cpu.A = 0xAE;

    writeByteToMemory(&mut mem, 0x2, 0xCCDD);

    //AE + 2 = B0
    let (newPC, cyclesTaken) = executeInstruction(0x86, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xB0);

    //H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    cpu.A = 0xFF;


    writeByteToMemory(&mut mem, 0x1, 0xCCDD);

    //FF + 1 = 0
    let (newPC, cyclesTaken) = executeInstruction(0x86, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0x0);

    //C, Z, H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn add8BitFromMem() { //0xC6

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;


    //memory has 0x11
    writeByteToMemory(&mut mem, 0x11, cpu.PC +1);

    //AA + 11 = BB
    let (newPC, cyclesTaken) = executeInstruction(0xC6, &mut cpu, &mut mem);

    assert_eq!(cyclesTaken, 8);
    assert_eq!(newPC, cpu.PC + 2);
    assert_eq!(cpu.A, 0xBB);

    //no flags set
    assert!(cpu.F == 0);


    cpu.A = 0xAE;

    writeByteToMemory(&mut mem, 0x2, cpu.PC +1);

    //AE + 2 = B0
    let (newPC, cyclesTaken) = executeInstruction(0xC6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0xB0);

    //H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    cpu.A = 0xFF;


    writeByteToMemory(&mut mem, 0x1, cpu.PC + 1);

    //FF + 1 = 0
    let (newPC, cyclesTaken) = executeInstruction(0xC6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0x0);

    //C, Z, H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn addAtoA() { //0x87

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0x66;



    //0x66 + 0x66 = 0xCC
    let (newPC, cyclesTaken) = executeInstruction(0x87, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xCC);

    //no flags set
    assert!(cpu.F == 0);


    cpu.A = 0x29;


    //0x29 + 0x29 = 0x52
    let (newPC, cyclesTaken) = executeInstruction(0x87, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0x52);

    //H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    cpu.A = 0x80;



    //0x80 + 0x80 = 0
    let (newPC, cyclesTaken) = executeInstruction(0x87, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0x0);

    //C, Z set
    assert!(!isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn addCarry8BitFromRegister() { //0x88-0x8D

    macro_rules! testAdd8 {
        //add registers
        ($srcReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            cpu.$srcReg = 0x11;

            //Make sure that adding carry works
            setFlag(Carry, &mut cpu.F);

            //AA + 11 + Carry = BC
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xBC);
            assert!(cpu.$srcReg == 0x11);

            //no flags set
            assert!(cpu.F == 0);


            cpu.A = 0xAE;

            cpu.$srcReg = 2;

            //AE + 2 = B0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xB0);

            //H set
            assert!(isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

            cpu.A = 0xFF;

            cpu.$srcReg = 1;

            //FF + 1 = 0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x0);

            //C, Z, H set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));


        });


    }


    testAdd8!(B, 0x88);
    testAdd8!(C, 0x89);
    testAdd8!(D, 0x8A);
    testAdd8!(E, 0x8B);
    testAdd8!(H, 0x8C);
    testAdd8!(L, 0x8D);
}

#[test]
fn addCarry8BitFromMemAtHL() { //0x8E

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    //HL has CCDD 
    cpu.H = 0xCC;
    cpu.L = 0xDD;

    //Make sure that adding carry works
    setFlag(Carry, &mut cpu.F);

    //memory has 0x11
    writeByteToMemory(&mut mem, 0x11, 0xCCDD);

    //AA + 11 + Carry = BC
    let (newPC, cyclesTaken) = executeInstruction(0x8E, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xBC);
    assert!(readByteFromMemory(&mem, 0xCCDD) == 0x11);

    //no flags set
    assert!(cpu.F == 0);


    cpu.A = 0xAE;

    writeByteToMemory(&mut mem, 0x2, 0xCCDD);

    //AE + 2 = B0
    let (newPC, cyclesTaken) = executeInstruction(0x8E, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xB0);

    //H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    cpu.A = 0xFF;


    writeByteToMemory(&mut mem, 0x1, 0xCCDD);

    //FF + 1 = 0
    let (newPC, cyclesTaken) = executeInstruction(0x8E, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0x0);

    //C, Z, H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn addCarry8BitFromMem() { //0xCE

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;


    //Make sure that adding carry works
    setFlag(Carry, &mut cpu.F);

    //memory has 0x11
    writeByteToMemory(&mut mem, 0x11, cpu.PC +1);

    //AA + 11 + Carry = BC
    let (newPC, cyclesTaken) = executeInstruction(0xCE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0xBC);

    //no flags set
    assert!(cpu.F == 0);


    cpu.A = 0xAE;

    writeByteToMemory(&mut mem, 0x2, cpu.PC +1);

    //AE + 2 = B0
    let (newPC, cyclesTaken) = executeInstruction(0xCE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0xB0);

    //H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    cpu.A = 0xFF;


    writeByteToMemory(&mut mem, 0x1, cpu.PC +1);

    //FF + 1 = 0
    let (newPC, cyclesTaken) = executeInstruction(0xCE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0x0);

    //C, Z, H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn addCarryAtoA() { //0x8F

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0x66;


    //Make sure that adding carry works
    setFlag(Carry, &mut cpu.F);

    //0x66 + 0x66 + Carry = 0xCD
    let (newPC, cyclesTaken) = executeInstruction(0x8F, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xCD);

    //no flags set
    assert!(cpu.F == 0);


    cpu.A = 0x29;


    //0x29 + 0x29 = 0x52
    let (newPC, cyclesTaken) = executeInstruction(0x8F, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0x52);

    //H set
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    cpu.A = 0x80;



    //0x80 + 0x80 = 0
    let (newPC, cyclesTaken) = executeInstruction(0x8F, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0x0);

    //C, Z set
    assert!(!isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}
#[test]
fn sub8Bit() { //0x90-0x95

    macro_rules! testSub8 {
        //add registers
        ($srcReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            //Make sure that sub carry is unaffected
            setFlag(Carry, &mut cpu.F);
            
            cpu.$srcReg = 0x11;

            //AA - 11 - Carry = 0x98
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x99);
            assert!(cpu.$srcReg == 0x11);

            //N flag set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0x1;

            cpu.$srcReg = 0xFF;

            //1 - FF = 2
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 2);

            //C, H, N set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0xAA;

            cpu.$srcReg = 0xAA;

            //AA - AA = 0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x0);

            //N, Z set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

        });


    }


    testSub8!(B, 0x90);
    testSub8!(C, 0x91);
    testSub8!(D, 0x92);
    testSub8!(E, 0x93);
    testSub8!(H, 0x94);
    testSub8!(L, 0x95);
}

#[test]
fn sub8BitFromMemAtHL() { //0x96

 
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            cpu.H = 0xCC;
            cpu.L = 0xDD;
            
            //Make sure that setting Carry doesn't affect anything
            setFlag(Carry, &mut cpu.F);

            writeByteToMemory(&mut mem, 0x11, 0xCCDD);

            //AA - 11 - Carry = 0x98
            let (newPC, cyclesTaken) = executeInstruction(0x96, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x99);

            //N flag set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0x1;

            writeByteToMemory(&mut mem, 0xFF, 0xCCDD);

            //1 - FF = 2
            let (newPC, cyclesTaken) = executeInstruction(0x96, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 2);

            //C, H, N set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0xAA;

            writeByteToMemory(&mut mem, 0xAA, 0xCCDD);
            
            //AA - AA = 0
            let (newPC, cyclesTaken) = executeInstruction(0x96, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x0);

            //N, Z set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));
 
}

#[test]
fn sub8BitFromMem() { //0x96

 
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;
            
            //Make sure that setting Carry doesn't affect anything
            setFlag(Carry, &mut cpu.F);

            writeByteToMemory(&mut mem, 0x11, cpu.PC +1);

            //AA - 11 - Carry = 0x98
            let (newPC, cyclesTaken) = executeInstruction(0xD6, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 2);
            assert!(cpu.A == 0x99);

            //N flag set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0x1;

            writeByteToMemory(&mut mem, 0xFF, cpu.PC +1);

            //1 - FF = 2
            let (newPC, cyclesTaken) = executeInstruction(0xD6, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 2);
            assert!(cpu.A == 2);

            //C, H, N set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0xAA;

            writeByteToMemory(&mut mem, 0xAA, cpu.PC +1);
            
            //AA - AA = 0
            let (newPC, cyclesTaken) = executeInstruction(0xD6, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 2);
            assert!(cpu.A == 0x0);

            //N, Z set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));
 
}

#[test]
fn subAFromA() { //0x97

   
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;
            //AA - AA = 0
            let (newPC, cyclesTaken) = executeInstruction(0x97, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x0);

            //N, Z, H set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));
 
}


#[test]
fn subCarry8Bit() { //0x98-0x9D

    macro_rules! testSubCarry8 {
        //add registers
        ($srcReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            //Make sure that we subtract carry
            setFlag(Carry, &mut cpu.F);
            
            cpu.A = 0xAA;

            cpu.$srcReg = 0x11;

            //AA - 11 = 0x99
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x98);
            assert!(cpu.$srcReg == 0x11);

            //N flag set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0x1;

            cpu.$srcReg = 0xFF;

            //1 - FF = 2
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 2);

            //C, H, N set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0xAA;

            cpu.$srcReg = 0xAA;

            //AA - AA = 0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xFF); //Carry is set in previous case

                //N,  H, C set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

        });


    }


    testSubCarry8!(B, 0x98);
    testSubCarry8!(C, 0x99);
    testSubCarry8!(D, 0x9A);
    testSubCarry8!(E, 0x9B);
    testSubCarry8!(H, 0x9C);
    testSubCarry8!(L, 0x9D);
}

#[test]
fn subCarry8BitFromMemAtHL() { //0x9E

 
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            cpu.H = 0xCC;
            cpu.L = 0xDD;

            writeByteToMemory(&mut mem, 0x11, 0xCCDD);

            //AA - 11 = 0x99
            let (newPC, cyclesTaken) = executeInstruction(0x9E, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x99);

            //N flag set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0x1;

            writeByteToMemory(&mut mem, 0xFF, 0xCCDD);

            //1 - FF = 2
            let (newPC, cyclesTaken) = executeInstruction(0x9E, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 2);

            //C, H, N set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0xAA;

            writeByteToMemory(&mut mem, 0xAA, 0xCCDD);
            
            //AA - AA - Carry = 0xFF
            let (newPC, cyclesTaken) = executeInstruction(0x9E, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xFF);

            //N, C, H set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));
 
}

#[test]
fn subCarry8BitFromMem() { //0xDE

 
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            writeByteToMemory(&mut mem, 0x11, cpu.PC + 1);

            //AA - 11 = 0x99
            let (newPC, cyclesTaken) = executeInstruction(0xDE, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 2);
            assert!(cpu.A == 0x99);

            //N flag set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0x1;

            writeByteToMemory(&mut mem, 0xFF, cpu.PC + 1);

            //1 - FF = 2
            let (newPC, cyclesTaken) = executeInstruction(0xDE, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 2);
            assert!(cpu.A == 2);

            //C, H, N set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0xAA;

            writeByteToMemory(&mut mem, 0xAA, cpu.PC + 1);
            
            //AA - AA - Carry = 0xFF
            let (newPC, cyclesTaken) = executeInstruction(0xDE, &mut cpu, &mut mem);

            assert!(cyclesTaken == 8);
            assert!(newPC == cpu.PC + 2);
            assert!(cpu.A == 0xFF);

            //N, C, H set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));
 
}

#[test]
fn subCarryAFromA() { //0x9F

   
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            //Make sure that adding carry works
            setFlag(Carry, &mut cpu.F);
            
            cpu.A = 0xAA;
            //AA - AA = 0
            let (newPC, cyclesTaken) = executeInstruction(0x9F, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xFF);

            //N, H, C set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));
 
}


#[test]
fn andRegToA() {

    macro_rules! testAnd {
        ($srcReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            cpu.$srcReg = 0x22;

            //AA & 22 = 22 
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0x22);
            assert!(cpu.$srcReg == 0x22);

            assert!(isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));


            cpu.A = 0xAA;

            cpu.$srcReg = 0x55;

            //AA & 0x55 = 0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0);

            //Z set
            assert!(isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

        });
    }


    testAnd!(B, 0xA0);
    testAnd!(C, 0xA1);
    testAnd!(D, 0xA2);
    testAnd!(E, 0xA3);
    testAnd!(H, 0xA4);
    testAnd!(L, 0xA5);

}

#[test]
fn andMemAtHLToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    cpu.H = 0xCC;
    cpu.L = 0xDD;


    writeByteToMemory(&mut mem, 0x22, 0xCCDD);


    //AA & 22 = 22 
    let (newPC, cyclesTaken) = executeInstruction(0xA6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0x22);
    assert!(readByteFromMemory(&mem, 0xCCDD) == 0x22);

    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));




    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0x55, 0xCCDD);

    //AA & 0x55 = 0
    let (newPC, cyclesTaken) = executeInstruction(0xA6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0);

    //Z set
    assert!(isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn andMemToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0x22, cpu.PC +1);


    //AA & 22 = 22 
    let (newPC, cyclesTaken) = executeInstruction(0xE6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0x22);

    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));




    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0x55, cpu.PC+1);

    //AA & 0x55 = 0
    let (newPC, cyclesTaken) = executeInstruction(0xE6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0);

    //Z set
    assert!(isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn andAToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;


    //AA & AA = AA
    let (newPC, cyclesTaken) = executeInstruction(0xA7, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xAA);

    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    cpu.A = 0;


    //0 & 0 = 0
    let (newPC, cyclesTaken) = executeInstruction(0xA7, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0);

    //Z set
    assert!(isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn xorRegToA() {

    macro_rules! testXOR {
        ($srcReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            cpu.$srcReg = 0xAA;

            //AA ^ AA = 0 
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0);
            assert!(cpu.$srcReg == 0xAA);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            //Z set
            assert!(isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));


            cpu.A = 0xAA;

            cpu.$srcReg = 0x55;

            //AA & 0x55 = 0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xFF);

            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

        });
    }


    testXOR!(B, 0xA8);
    testXOR!(C, 0xA9);
    testXOR!(D, 0xAA);
    testXOR!(E, 0xAB);
    testXOR!(H, 0xAC);
    testXOR!(L, 0xAD);

}

#[test]
fn xorMemAtHLToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    cpu.H = 0xCC;
    cpu.L = 0xDD;


    writeByteToMemory(&mut mem, 0xAA, 0xCCDD);


    //AA ^ AA = 0
    let (newPC, cyclesTaken) = executeInstruction(0xAE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0);
    assert!(readByteFromMemory(&mem, 0xCCDD) == 0xAA);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    
    //Z set
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));




    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0x55, 0xCCDD);

    //AA ^ 0x55 =  0xFF
    let (newPC, cyclesTaken) = executeInstruction(0xAE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xFF);

    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn xorMemToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0xAA, cpu.PC + 1);

    //AA ^ AA = 0
    let (newPC, cyclesTaken) = executeInstruction(0xEE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    
    //Z set
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0x55, cpu.PC + 1);

    //AA ^ 0x55 =  0xFF
    let (newPC, cyclesTaken) = executeInstruction(0xEE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0xFF);

    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn xorAToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;


    //AA ^ AA = 0
    let (newPC, cyclesTaken) = executeInstruction(0xAF, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn orRegToA() {

    macro_rules! testOR {
        ($srcReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();


            //0 | 0 = 0
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            //Z set
            assert!(isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));


            cpu.A = 0xAA;

            cpu.$srcReg = 0xFF;

            //AA | 0xFF = 0xFF
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xFF);

            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));

        });
    }


    testOR!(B, 0xB0);
    testOR!(C, 0xB1);
    testOR!(D, 0xB2);
    testOR!(E, 0xB3);
    testOR!(H, 0xB4);
    testOR!(L, 0xB5);

}

#[test]
fn orMemAtHLToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0;

    cpu.H = 0xCC;
    cpu.L = 0xDD;


    writeByteToMemory(&mut mem, 0, 0xCCDD);


    //0 | 0 = 0
    let (newPC, cyclesTaken) = executeInstruction(0xB6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    
    //Z set
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));




    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0xFF, 0xCCDD);

    //AA | 0xFF =  0xFF
    let (newPC, cyclesTaken) = executeInstruction(0xB6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xFF);

    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn orMemLToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0;

    cpu.H = 0xCC;
    cpu.L = 0xDD;


    writeByteToMemory(&mut mem, 0, cpu.PC +1);


    //0 | 0 = 0
    let (newPC, cyclesTaken) = executeInstruction(0xF6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    
    //Z set
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));




    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0xFF, cpu.PC + 1);

    //AA | 0xFF =  0xFF
    let (newPC, cyclesTaken) = executeInstruction(0xF6, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0xFF);

    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn orAToA() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;


    //AA | AA = AA
    let (newPC, cyclesTaken) = executeInstruction(0xB7, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xAA);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));

}

#[test]
fn compare8Bit() { //0xB8-0xBD

    macro_rules! testCompare8 {
        //add registers
        ($srcReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.A = 0xAA;

            //Make sure that sub carry is unaffected
            setFlag(Carry, &mut cpu.F);
            
            cpu.$srcReg = 0x11;

            //AA > 11, C and Z clear
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            //A is unchanged
            assert!(cpu.A == 0xAA);
            assert!(cpu.$srcReg == 0x11);

            //N flag set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0x1;

            cpu.$srcReg = 0xFF;

            //1 < FF, C set Z clear  
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 1);

            //C, H, N set
            assert!(isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

            cpu.A = 0xAA;

            cpu.$srcReg = 0xAA;

            //AA == AA, Z set, C clear
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert!(cyclesTaken == 4);
            assert!(newPC == cpu.PC + 1);
            assert!(cpu.A == 0xAA);

            //N, Z set
            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(isFlagSet(Neg, cpu.F));

        });


    }


    testCompare8!(B, 0xB8);
    testCompare8!(C, 0xB9);
    testCompare8!(D, 0xBA);
    testCompare8!(E, 0xBB);
    testCompare8!(H, 0xBC);
    testCompare8!(L, 0xBD);
}

#[test]
fn compare8BitFromMemAtHL() { //0xBE


    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    cpu.H = 0xCC;
    cpu.L = 0xDD;

    //Make sure that setting Carry doesn't affect anything
    setFlag(Carry, &mut cpu.F);

    writeByteToMemory(&mut mem, 0x11, 0xCCDD);

    //AA > 11, C and Z clear
    let (newPC, cyclesTaken) = executeInstruction(0xBE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xAA);

    //N flag set
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

    cpu.A = 0x1;

    writeByteToMemory(&mut mem, 0xFF, 0xCCDD);

    //1 < FF, C set Z clear  
    let (newPC, cyclesTaken) = executeInstruction(0xBE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 1);

    //C, H, N set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0xAA, 0xCCDD);

    //AA == AA, Z set, C clear
    let (newPC, cyclesTaken) = executeInstruction(0xBE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xAA);

    //N, Z set
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

}

#[test]
fn compare8BitFromMem() { //0xFE


    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    //Make sure that setting Carry doesn't affect anything
    setFlag(Carry, &mut cpu.F);

    writeByteToMemory(&mut mem, 0x11, cpu.PC + 1);

    //AA > 11, C and Z clear
    let (newPC, cyclesTaken) = executeInstruction(0xFE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0xAA);

    //N flag set
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

    cpu.A = 0x1;

    writeByteToMemory(&mut mem, 0xFF, cpu.PC + 1);

    //1 < FF, C set Z clear  
    let (newPC, cyclesTaken) = executeInstruction(0xFE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 1);

    //C, H, N set
    assert!(isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0xAA, cpu.PC + 1);

    //AA == AA, Z set, C clear
    let (newPC, cyclesTaken) = executeInstruction(0xFE, &mut cpu, &mut mem);

    assert!(cyclesTaken == 8);
    assert!(newPC == cpu.PC + 2);
    assert!(cpu.A == 0xAA);

    //N, Z set
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));

}

#[test]
fn compareAToA() { //0xBF


    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    //AA == AA, Z set, C clear
    let (newPC, cyclesTaken) = executeInstruction(0xBF, &mut cpu, &mut mem);

    assert!(cyclesTaken == 4);
    assert!(newPC == cpu.PC + 1);
    assert!(cpu.A == 0xAA);

    //N, Z, H set
    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(isFlagSet(Neg, cpu.F));
 
}

#[test]
fn callAndReturn() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    let oldPC = cpu.PC;

    writeWordToMemory(&mut mem, 0xCC00, cpu.PC+1); //call address 0xCC00

    //execute CALL a16
    let (newPC, cyclesTaken) = executeInstruction(0xCD, &mut cpu, &mut mem);

    assert_eq!(newPC, 0xCC00);
    assert_eq!(readWordFromMemory(&mut mem, cpu.SP), oldPC + 3);
    assert_eq!(cyclesTaken, 24);

    //execute RET
    let (newPC, cyclesTaken) = executeInstruction(0xC9, &mut cpu, &mut mem);
    
    assert_eq!(newPC, oldPC + 3);
    assert_eq!(readWordFromMemory(&mut mem, cpu.SP), 0);
    assert_eq!(cyclesTaken, 16);

}

#[test]
fn restart() {

    macro_rules! testRestart {
        ($resetAddress: expr, $inst:expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            let oldPC = cpu.PC;

            //execute CALL a16
            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert_eq!(newPC, $resetAddress);
            assert_eq!(readWordFromMemory(&mut mem, cpu.SP), oldPC + 1);
            assert_eq!(cyclesTaken, 16);
        })
    }

    testRestart!(0x0, 0xC7);
    testRestart!(0x8, 0xCF);
    testRestart!(0x10, 0xD7);
    testRestart!(0x18, 0xDF);
    testRestart!(0x20, 0xE7);
    testRestart!(0x28, 0xEF);
    testRestart!(0x30, 0xF7);
    testRestart!(0x38, 0xFF);

}

#[test]
fn returnFromProcConditional() {

    fn testRET(flag: Flag, shouldBeSet: bool, inst: u8) {
        let mut cpu = testingCPU();
        let mut mem = tetrisMemoryState();

        let mut oldPC = cpu.PC;

        writeWordToMemory(&mut mem, 0xCC00, cpu.PC+1); //call address 0xCC00

        //execute CALL a16
        executeInstruction(0xCD, &mut cpu, &mut mem);

        if shouldBeSet {
            setFlag(flag, &mut cpu.F);
        }
        else {
            clearFlag(flag, &mut cpu.F);
        }


        //execute RET
        let (newPC, cyclesTaken) = executeInstruction(inst, &mut cpu, &mut mem);

        assert_eq!(newPC, oldPC + 3);
        assert_eq!(readWordFromMemory(&mut mem, cpu.SP), 0);
        assert_eq!(cyclesTaken, 20);



        //should not perform jump
        if !shouldBeSet {
            setFlag(flag, &mut cpu.F);
        }
        else {
            clearFlag(flag, &mut cpu.F);
        }

        oldPC = cpu.PC;

        //execute CALL a16
        executeInstruction(0xCD, &mut cpu, &mut mem);

        //execute RET
        let (newPC, cyclesTaken) = executeInstruction(inst, &mut cpu, &mut mem);

        assert_eq!(newPC, cpu.PC + 1);
        assert_eq!(readWordFromMemory(&mut mem, cpu.SP), oldPC + 3);  //make sure the return address is still on the stack
        assert_eq!(cyclesTaken, 8);
    }

    testRET(Zero, false, 0xC0);
    testRET(Zero, true, 0xC8);
    testRET(Carry, false, 0xD0);
    testRET(Carry, true, 0xD8);
    

}

fn returnFromInterruptProc() {
    //TODO(DanB): To be tested once interrupts are implemented
}

#[test]
fn callConditional() {

    fn testCALL(flag: Flag, shouldBeSet: bool, inst: u8) {
        let mut cpu = testingCPU();
        let mut mem = tetrisMemoryState();

        let oldPC = cpu.PC;

        writeWordToMemory(&mut mem, 0xCC00, cpu.PC+1); //call address 0xCC00

        if shouldBeSet {
            setFlag(flag, &mut cpu.F);
        }
        else {
            clearFlag(flag, &mut cpu.F);
        }


        //execute CALL CC00
        let (newPC, cyclesTaken) = executeInstruction(inst, &mut cpu, &mut mem);

        assert_eq!(newPC, 0xCC00);
        assert_eq!(readWordFromMemory(&mut mem, cpu.SP), oldPC + 3);
        assert_eq!(cyclesTaken, 24);



        //should not perform jump
        if !shouldBeSet {
            setFlag(flag, &mut cpu.F);
        }
        else {
            clearFlag(flag, &mut cpu.F);
        }


        //execute CALL CC00
        let (newPC, cyclesTaken) = executeInstruction(inst, &mut cpu, &mut mem);

        assert_eq!(newPC, cpu.PC + 3);
        assert_eq!(cyclesTaken, 12);
    }

    testCALL(Zero, false, 0xC4);
    testCALL(Zero, true, 0xCC);

    testCALL(Carry, false, 0xD4);
    testCALL(Carry, true, 0xDC);
}

#[test]
fn pop16() {
    macro_rules! testPop16 {
        ($highReg: ident, $lowReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            writeWordToMemory(&mut mem, 0xAABB, 0xCCD0);
            cpu.SP = 0xCCD0;

            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert_eq!(cpu.SP, 0xCCD2);
            assert_eq!(word(cpu.$highReg, cpu.$lowReg), 0xAABB);
            assert_eq!(cyclesTaken, 12);
            assert_eq!(newPC, cpu.PC+1);

        })
    }

   testPop16!(B,C,0xC1); 
   testPop16!(D,E,0xD1); 
   testPop16!(H,L,0xE1); 
   testPop16!(A,F,0xF1); 
}

#[test]
fn push16() {
    macro_rules! testPush16 {
        ($highReg: ident, $lowReg: ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            cpu.SP = 0xCCD0;

            //push AABB on to stack
            cpu.$highReg = 0xAA;
            cpu.$lowReg = 0xBB;


            let (newPC, cyclesTaken) = executeInstruction($inst, &mut cpu, &mut mem);

            assert_eq!(cpu.SP, 0xCCCE);
            assert_eq!(readWordFromMemory(&mut mem, cpu.SP), 0xAABB);
            assert_eq!(cyclesTaken, 16);
            assert_eq!(newPC, cpu.PC+1);

        })
    }

    testPush16!(B,C,0xC5); 
    testPush16!(D,E,0xD5); 
    testPush16!(H,L,0xE5); 
    testPush16!(A,F,0xF5); 

}


#[test]
fn jumpAbsoluteConditional() { 

    fn testJRC(flag: Flag, shouldBeSet: bool, inst: u8) {
        let mut cpu = testingCPU();
        let mut mem = tetrisMemoryState();

        //should perform jump
        if shouldBeSet {
            setFlag(flag, &mut cpu.F);
        }
        else {
            clearFlag(flag, &mut cpu.F);
        }

        //load address to jump to 
        writeWordToMemory(&mut mem, 0xAABB, cpu.PC+1);

        let (newPC, cyclesTaken) = executeInstruction(inst, &mut cpu, &mut mem);

        assert_eq!(newPC, 0xAABB);
        assert_eq!(cyclesTaken, 16);

        //should not perform jump
        if !shouldBeSet {
            setFlag(flag, &mut cpu.F);
        }
        else {
            clearFlag(flag, &mut cpu.F);
        }

        //load address 
        writeWordToMemory(&mut mem, 0xAABB, cpu.PC+1);

        let (newPC, cyclesTaken) = executeInstruction(inst, &mut cpu, &mut mem);

        assert_eq!(newPC, cpu.PC + 3);
        assert_eq!(cyclesTaken, 12);

    }

    testJRC(Zero, false, 0xC2);
    testJRC(Zero, true, 0xCA);
    testJRC(Carry, false, 0xD2);
    testJRC(Carry, true, 0xDA);


}

#[test]
fn jumpAbsolute() {
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    writeWordToMemory(&mut mem, 0xAABB, cpu.PC+1);
    
    let (newPC, cyclesTaken) = executeInstruction(0xC3, &mut cpu, &mut mem);

    assert_eq!(newPC, 0xAABB);
    assert_eq!(cyclesTaken, 16);
}

#[test]
fn loadAIntoHighMem() { //E0
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    writeByteToMemory(&mut mem, 0xBB, cpu.PC+1); 

    let (newPC, cyclesTaken) = executeInstruction(0xE0, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xFFBB), 0xAA);
    assert_eq!(newPC, cpu.PC + 2);
    assert_eq!(cyclesTaken, 12);

}

#[test]
fn loadAIntoHighMemAtC() { //E2
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    //load AA into FFBB
    cpu.C = 0xBB;
    let (newPC, cyclesTaken) = executeInstruction(0xE2, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xFFBB), 0xAA);
    assert_eq!(newPC, cpu.PC + 1);
    assert_eq!(cyclesTaken, 8);

}

#[test]
fn addToSPSigned() { //E8

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    let oldSP = cpu.SP;

    writeByteToMemory(&mut mem, -2i8 as u8, cpu.PC+1);
    cpu.F = 0xF0; //set all the flags

    let (newPC, cyclesTaken) = executeInstruction(0xE8, &mut cpu, &mut mem);

    assert_eq!(cpu.SP, oldSP -2);
    assert_eq!(newPC, cpu.PC + 2);
    assert_eq!(cyclesTaken, 16);

    assert_eq!(cpu.F, 0);
    
    cpu.SP = 0xFEF8;

    writeByteToMemory(&mut mem, 0x8, cpu.PC+1);
    cpu.F = 0xF0; //set all the flags

    let (newPC, cyclesTaken) = executeInstruction(0xE8, &mut cpu, &mut mem);

    assert_eq!(cpu.SP, 0xFF00);
    assert_eq!(newPC, cpu.PC + 2);
    assert_eq!(cyclesTaken, 16);

    //Carry and Half set
    assert!(isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn jumpUsingHL() {//E9
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.H = 0xAA;
    cpu.L = 0xBB;

    //jump to 0xAABB
    let (newPC, cyclesTaken) = executeInstruction(0xE9, &mut cpu, &mut mem);
    
    assert_eq!(newPC, 0xAABB);
    assert_eq!(cyclesTaken, 4);

}

#[test]
fn loadAIntoMem() { //EA
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    writeWordToMemory(&mut mem, 0xCCBB, cpu.PC+1); 

    let (newPC, cyclesTaken) = executeInstruction(0xEA, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xCCBB), 0xAA);
    assert_eq!(newPC, cpu.PC + 3);
    assert_eq!(cyclesTaken, 16);

}

#[test]
fn loadHighMemIntoA() { //F0
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();


    writeByteToMemory(&mut mem, 0xBB, cpu.PC+1); 

    //A will have 0xAA
    writeByteToMemory(&mut mem, 0xAA, 0xFFBB); 

    let (newPC, cyclesTaken) = executeInstruction(0xF0, &mut cpu, &mut mem);

    assert_eq!(cpu.A, 0xAA);
    assert_eq!(newPC, cpu.PC + 2);
    assert_eq!(cyclesTaken, 12);

}

#[test]
fn loadHighMemAtCIntoA() { //F2
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.C = 0xBB;

    //A will have 0xAA
    writeByteToMemory(&mut mem, 0xAA, 0xFFBB); 

    let (newPC, cyclesTaken) = executeInstruction(0xF2, &mut cpu, &mut mem);

    assert_eq!(cpu.A, 0xAA);
    assert_eq!(newPC, cpu.PC + 2);
    assert_eq!(cyclesTaken, 12);

}

fn disableInterrupts() {
    //TODO: Write test once interrupts are impmeneted
}

#[test]
fn loadSPPlusImmIntoSP() { //F8

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.SP = 0xFF00;
    writeByteToMemory(&mut mem, -2i8 as u8, cpu.PC+1);
    cpu.F = 0xF0; //set all the flags

    let (newPC, cyclesTaken) = executeInstruction(0xF8, &mut cpu, &mut mem);

    assert_eq!(word(cpu.H, cpu.L), 0xFEFE);
    assert_eq!(newPC, cpu.PC + 2);
    assert_eq!(cyclesTaken, 12);

    assert_eq!(cpu.F, 0);
    
    cpu.SP = 0xFEF8;

    writeByteToMemory(&mut mem, 0x8, cpu.PC+1);
    cpu.F = 0xF0; //set all the flags

    let (newPC, cyclesTaken) = executeInstruction(0xF8, &mut cpu, &mut mem);

    assert_eq!(word(cpu.H, cpu.L), 0xFF00);
    assert_eq!(newPC, cpu.PC + 2);
    assert_eq!(cyclesTaken, 12);

    //Carry and Half set
    assert!(isFlagSet(Carry, cpu.F));
    assert!(isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
}

#[test]
fn loadHLIntoSP() { //F9
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.H = 0xAA;
    cpu.L = 0xBB;

    let (newPC, cyclesTaken) = executeInstruction(0xF9, &mut cpu, &mut mem);

    assert_eq!(cpu.SP, 0xAABB);
    assert_eq!(newPC, cpu.PC + 1);
    assert_eq!(cyclesTaken, 8);
}

#[test]
fn loadMemIntoA() { //FA
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();

    cpu.A = 0xAA;

    writeWordToMemory(&mut mem, 0xCCBB, cpu.PC+1); 
    writeByteToMemory(&mut mem, 0xAA, 0xCCBB); 

    let (newPC, cyclesTaken) = executeInstruction(0xFA, &mut cpu, &mut mem);

    assert_eq!(cpu.A, 0xAA);
    assert_eq!(newPC, cpu.PC + 3);
    assert_eq!(cyclesTaken, 16);

}

fn enableInterrupts() {
    //TODO: Test once interrupts are implemented
}

#[test]
fn rotateLeftCB() { //0xCB00-0xCB07

    macro_rules! testRLC {
        ($reg:ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();


            writeByteToMemory(&mut mem, $inst, cpu.PC + 1);

            //test rotate 0
            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(cpu.$reg == 0);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));

            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 8);

            //test C set
            cpu.$reg = 0x88;


            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(cpu.$reg == 0x11);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));


            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 8);

            //test C clear
            cpu.$reg = 0x7F;

            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(cpu.$reg == 0xFE);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));


            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 8);

        })
    }

    testRLC!(B, 0);
    testRLC!(C, 1);
    testRLC!(D, 2);
    testRLC!(E, 3);
    testRLC!(H, 4);
    testRLC!(L, 5);
    testRLC!(A, 7);

}

#[test]
fn rotateLeftCBAtHL() {

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();


    writeByteToMemory(&mut mem, 6, cpu.PC + 1);


    //Byte at CCBB
    cpu.H = 0xCC;
    cpu.L = 0xBB;

    //test rotate 0
    let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xCCBB), 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));

    assert!(newPC == cpu.PC + 2);
    assert!(cyclesTaken == 16);

    //test C set
    writeByteToMemory(&mut mem, 0x88, 0xCCBB);


    let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xCCBB), 0x11);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));


    assert!(newPC == cpu.PC + 2);
    assert!(cyclesTaken == 16);

    //test C clear
    writeByteToMemory(&mut mem, 0x7F, 0xCCBB);

    let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xCCBB), 0xFE);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));

    assert!(newPC == cpu.PC + 2);
    assert!(cyclesTaken == 16);
}

#[test]
fn rotateRightCB() { //CB08 - CB0D and CB0F

    macro_rules! testRRC {
        ($reg:ident, $inst: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();


            writeByteToMemory(&mut mem, $inst, cpu.PC + 1);
            //test rotate 0
            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(cpu.$reg == 0);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));

            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 8);

            //test C set
            cpu.$reg = 0x11;


            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(cpu.$reg == 0x88);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));
            assert!(isFlagSet(Carry, cpu.F));


            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 8);

            //test C clear
            cpu.$reg = 0x76;

            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(cpu.$reg == 0x3B);

            assert!(!isFlagSet(Half, cpu.F));
            assert!(!isFlagSet(Zero, cpu.F));
            assert!(!isFlagSet(Neg, cpu.F));
            assert!(!isFlagSet(Carry, cpu.F));


            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 8);

        })
    }

    testRRC!(B, 8);
    testRRC!(C, 9);
    testRRC!(D, 0xA);
    testRRC!(E, 0xB);
    testRRC!(H, 0xC);
    testRRC!(L, 0xD);
    testRRC!(A, 0xF);
}



#[test]
fn rotateRightCBAtHL() {//E

    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();


    writeByteToMemory(&mut mem, 0xE, cpu.PC + 1);


    //Byte at CCBB
    cpu.H = 0xCC;
    cpu.L = 0xBB;

    //test rotate 0
    let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xCCBB), 0);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));

    assert!(newPC == cpu.PC + 2);
    assert!(cyclesTaken == 16);

    //test C set
    writeByteToMemory(&mut mem, 0x11, 0xCCBB);


    let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xCCBB), 0x88);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(isFlagSet(Carry, cpu.F));


    assert!(newPC == cpu.PC + 2);
    assert!(cyclesTaken == 16);

    //test C clear
    writeByteToMemory(&mut mem, 0x76, 0xCCBB);

    let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

    assert_eq!(readByteFromMemory(&mut mem, 0xCCBB), 0x3B);

    assert!(!isFlagSet(Half, cpu.F));
    assert!(!isFlagSet(Zero, cpu.F));
    assert!(!isFlagSet(Neg, cpu.F));
    assert!(!isFlagSet(Carry, cpu.F));

    assert!(newPC == cpu.PC + 2);
    assert!(cyclesTaken == 16);
}

#[test]
fn rotateLeftThroughCarryCB() { //CB10 - CB15 and CB17

    macro_rules! testRLA {
        ($regAVal: expr, $expectedVal: expr, $setC: expr, $isCSet: expr, $reg:ident, $inst:expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            writeByteToMemory(&mut mem, $inst, cpu.PC + 1);

            cpu.$reg = $regAVal;

            if $setC {
                setFlag(Carry, &mut cpu.F);
            }

            //test rotate 0
            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(cpu.$reg == $expectedVal);

            assert!(!isFlagSet(Half, cpu.F));
            
            if $expectedVal == 0 {
                assert!(isFlagSet(Zero, cpu.F));
            }
            else {
                assert!(!isFlagSet(Zero, cpu.F));
            }

            assert!(!isFlagSet(Neg, cpu.F));

            if $isCSet {
                assert!(isFlagSet(Carry, cpu.F));
            }
            else {
                assert!(!isFlagSet(Carry, cpu.F));
            }

            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 8);


        })
    }

    macro_rules! runTestCases {
        ($reg: ident, $inst:expr) => ({

            //test rotate 0
            testRLA!(0, 0, false, false, $reg, $inst);

            //test C will be set
            testRLA!(0x88, 0x10, false, true, $reg, $inst);

            //test C clear
            testRLA!(0x7F, 0xFE, false, false, $reg, $inst);

            //test with C already set
            testRLA!(0x80, 0x1, true, true, $reg, $inst);
        })
    }

    runTestCases!(B, 0x10);
    runTestCases!(C, 0x11);
    runTestCases!(D, 0x12);
    runTestCases!(E, 0x13);
    runTestCases!(H, 0x14);
    runTestCases!(L, 0x15);
    runTestCases!(A, 0x17);

}
#[test]
fn rotateLeftThroughCarryAtHLCB() { //CB16
    macro_rules! testRLA {
        ($regAVal: expr, $expectedVal: expr, $setC: expr, $isCSet: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            writeByteToMemory(&mut mem, 0x16, cpu.PC+1);
            writeByteToMemory(&mut mem, $regAVal, 0xCCBB);


            cpu.H = 0xCC;
            cpu.L = 0xBB;

            if $setC {
                setFlag(Carry, &mut cpu.F);
            }

            //test rotate 0
            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(readByteFromMemory(&mut mem, 0xCCBB) == $expectedVal);

            assert!(!isFlagSet(Half, cpu.F));
            
            if $expectedVal == 0 {
                assert!(isFlagSet(Zero, cpu.F));
            }
            else {
                assert!(!isFlagSet(Zero, cpu.F));
            }

            assert!(!isFlagSet(Neg, cpu.F));

            if $isCSet {
                assert!(isFlagSet(Carry, cpu.F));
            }
            else {
                assert!(!isFlagSet(Carry, cpu.F));
            }

            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 16);


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
fn rotateRightThroughCarryCB() { //CB18 - CB1D and CB1F

    macro_rules! testRR {
        ($regAVal: expr, $expectedVal: expr, $setC: expr, $isCSet: expr, $reg:ident, $inst:expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            writeByteToMemory(&mut mem, $inst, cpu.PC + 1);

            cpu.$reg = $regAVal;

            if $setC {
                setFlag(Carry, &mut cpu.F);
            }

            //test rotate 0
            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(cpu.$reg == $expectedVal);

            assert!(!isFlagSet(Half, cpu.F));
            
            if $expectedVal == 0 {
                assert!(isFlagSet(Zero, cpu.F));
            }
            else {
                assert!(!isFlagSet(Zero, cpu.F));
            }

            assert!(!isFlagSet(Neg, cpu.F));

            if $isCSet {
                assert!(isFlagSet(Carry, cpu.F));
            }
            else {
                assert!(!isFlagSet(Carry, cpu.F));
            }

            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 8);


        })
    }

    macro_rules! runTestCases {
        ($reg: ident, $inst:expr) => ({

            //test rotate 0
            testRR!(0, 0, false, false, $reg, $inst);

            //test C will be set
            testRR!(0x11, 0x8, false, true, $reg, $inst);

            //test C clear
            testRR!(0x88, 0x44, false, false, $reg, $inst);

            //test with C already set
            testRR!(0x81, 0xC0, true, true, $reg, $inst);
        })
    }

    runTestCases!(B, 0x18);
    runTestCases!(C, 0x19);
    runTestCases!(D, 0x1A);
    runTestCases!(E, 0x1B);
    runTestCases!(H, 0x1C);
    runTestCases!(L, 0x1D);
    runTestCases!(A, 0x1F);

}

#[test]
fn rotateRightThroughCarryAtHLCB() { //CB1E
    macro_rules! testRR {
        ($regAVal: expr, $expectedVal: expr, $setC: expr, $isCSet: expr) => ({
            let mut cpu = testingCPU();
            let mut mem = tetrisMemoryState();

            writeByteToMemory(&mut mem, 0x1E, cpu.PC+1);
            writeByteToMemory(&mut mem, $regAVal, 0xCCBB);


            cpu.H = 0xCC;
            cpu.L = 0xBB;

            if $setC {
                setFlag(Carry, &mut cpu.F);
            }

            //test rotate 0
            let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);

            assert!(readByteFromMemory(&mut mem, 0xCCBB) == $expectedVal);

            assert!(!isFlagSet(Half, cpu.F));
            
            if $expectedVal == 0 {
                assert!(isFlagSet(Zero, cpu.F));
            }
            else {
                assert!(!isFlagSet(Zero, cpu.F));
            }

            assert!(!isFlagSet(Neg, cpu.F));

            if $isCSet {
                assert!(isFlagSet(Carry, cpu.F));
            }
            else {
                assert!(!isFlagSet(Carry, cpu.F));
            }

            assert!(newPC == cpu.PC + 2);
            assert!(cyclesTaken == 16);


        })
    }


    //test rotate 0
    testRR!(0, 0, false, false);

    //test C will be set
    testRR!(0x11, 0x8, false, true);

    //test C clear
    testRR!(0x88, 0x44, false, false);

    //test with C already set
    testRR!(0x81, 0xC0, true, true);


}


//tests all CB instructions except rotate instructions
//TODO: Perhaps merge rotate instructions into here....
#[test]
fn cbInstructions() {
    let mut cpu = testingCPU();
    let mut mem = tetrisMemoryState();


    //inner scope for macros to work
    {
        macro_rules! storeValue {
            ($value: expr, $inst: expr) => ({
                match $inst  % 8  {
                    0 => cpu.B = $value,
                    1 => cpu.C = $value,
                    2 => cpu.D = $value,
                    3 => cpu.E = $value,
                    4 => cpu.H = $value,
                    5 => cpu.L = $value,
                    // we will be using address 0xCCBB to store (HL) values
                    6 =>{
                        cpu.H = 0xCC;
                        cpu.L = 0xBB;
                        writeByteToMemory(&mut mem, $value, 0xCCBB);
                    },    
                    7 => cpu.A = $value,
                    _ => panic!("Unreachable")
                }
            })
        }

        macro_rules! testValue {
            ($value: expr, $inst: expr) => ({
                let toTest = match $inst % 8  {
                    0 => cpu.B,
                    1 => cpu.C,
                    2 => cpu.D,
                    3 => cpu.E,
                    4 => cpu.H,
                    5 => cpu.L,
                    // we will be using address 0xCCBB to store (HL) values
                    6 => readByteFromMemory(&mut mem, 0xCCBB),
                    7 => cpu.A,
                    _ => panic!("Unreachable")
                
                };


                assert_eq!(toTest, $value);
                println!("Testing value: {:X} passed!", $value);
            })
        }

        macro_rules! testCyclesTaken {
            ($value: expr, $inst: expr) => ({
                if ($inst & 0xF) % 8 == 6 {
                    assert_eq!($value, 16);
                }
                else {
                    assert_eq!($value, 8);
                }
                println!("Cycles taken test passed!");
            })

        }


        for i in 0x20..0xFF {
            println!("Testing Instruction: {:X}", i); 
            writeByteToMemory(&mut mem, i, cpu.PC + 1);
            cpu.F = 0xF0; //set all flags

            macro_rules! executeInstruction {
                () => ({
                    let (newPC, cyclesTaken) = executeInstruction(0xCB, &mut cpu, &mut mem);
                    testCyclesTaken!(cyclesTaken, i);
                    assert_eq!(newPC, cpu.PC + 2);

                })
            }

            match i {
                0x20...0x27 => { //SLA

                    //-------------------test case 1-----------------------------------------
                    storeValue!(0x80, i);
                    executeInstruction!();
                    testValue!(0, i);

                    //Z and C set
                    assert!(isFlagSet(Zero, cpu.F));
                    assert!(isFlagSet(Carry, cpu.F));
                    assert!(!isFlagSet(Neg, cpu.F));
                    assert!(!isFlagSet(Half, cpu.F));
                    //----------------------------------------------------------------------


                    //-------------------test case 2-----------------------------------------
                    //0x11 shifted left is 0x22
                    storeValue!(0x11, i);
                    executeInstruction!();
                    testValue!(0x22, i);
                    //no flags set
                    assert_eq!(0, cpu.F);
                    //----------------------------------------------------------------------
                }

                0x28...0x2F => { //SRA

                    //-------------------test case 1-----------------------------------------
                    storeValue!(0x80, i);
                    executeInstruction!();

                    testValue!(0xC0, i); //propagated sign bit

                    //no flags set
                    assert_eq!(0, cpu.F);
                    //----------------------------------------------------------------------
                    
                    
                    //------------------test case 2----------------------------------------
                    //0x1 shifted right is 0
                    storeValue!(1, i);
                    executeInstruction!();
                    testValue!(0, i);
                    //Z set
                    assert!(isFlagSet(Zero, cpu.F));
                    assert!(isFlagSet(Carry, cpu.F));
                    assert!(!isFlagSet(Neg, cpu.F));
                    assert!(!isFlagSet(Half, cpu.F));
                    //--------------------------------------------------------------------

                }

                0x30...0x37 => { //SWAP
                    //-------------------test case 1-----------------------------------------
                    storeValue!(0x80, i);

                    executeInstruction!();
                    testValue!(0x08, i);

                    //no flags set
                    assert_eq!(0, cpu.F);
                    //----------------------------------------------------------------------
                   

                    //-------------------test case 2-----------------------------------------
                    storeValue!(0, i);

                    executeInstruction!();
                    testValue!(0, i);
                    //Z set
                    assert!(isFlagSet(Zero, cpu.F));
                    assert!(!isFlagSet(Carry, cpu.F));
                    assert!(!isFlagSet(Neg, cpu.F));
                    assert!(!isFlagSet(Half, cpu.F));
                    //----------------------------------------------------------------------

                },

                0x38...0x3F => { //SRL
                    //-------------------test case 1-----------------------------------------
                    storeValue!(0x80, i);
                    executeInstruction!();

                    testValue!(0x40, i); //don't propagate sign bit

                    //no flags set
                    assert_eq!(0, cpu.F);
                    //----------------------------------------------------------------------
                    
                    
                    //------------------test case 2----------------------------------------
                    //0x1 shifted right is 0
                    storeValue!(1, i);
                    executeInstruction!();
                    testValue!(0, i);
                    //Z set
                    assert!(isFlagSet(Zero, cpu.F));
                    assert!(isFlagSet(Carry, cpu.F));
                    assert!(!isFlagSet(Neg, cpu.F));
                    assert!(!isFlagSet(Half, cpu.F));
                    //--------------------------------------------------------------------

                }
                0x40...0x7F => { //BIT
                    macro_rules! testBit {
                        ($value: expr, $shouldZeroBeSet: expr)=>  ({
                            storeValue!($value, i);
                            executeInstruction!();

                            assert!(!isFlagSet(Neg, cpu.F));
                            assert!(isFlagSet(Half, cpu.F));

                            if $shouldZeroBeSet {
                                assert!(isFlagSet(Zero, cpu.F));
                            }
                            else {
                                assert!(!isFlagSet(Zero, cpu.F));
                            }

                        })
                    }
                    match i {

                        0x40...0x47 => {//BIT 0
                            testBit!(0x80, true);
                            testBit!(0x81, false);
                        }

                        0x48...0x4F => {//BIT 1
                            testBit!(0x81, true);
                            testBit!(0x83, false);
                        }
                        
                        0x50...0x57 => {//BIT 2
                            testBit!(0x83, true);
                            testBit!(0x87, false);
                        }
                        
                        0x58...0x5F => {//BIT 3
                            testBit!(0x87, true);
                            testBit!(0x8F, false);
                        }

                        0x60...0x67 => {//BIT 4
                            testBit!(0x8F, true);
                            testBit!(0x90, false);
                        }

                        0x68...0x6F => {//BIT 5
                            testBit!(0x90, true);
                            testBit!(0x20, false);
                        }
                        
                        0x70...0x77 => {//BIT 6
                            testBit!(0x20, true);
                            testBit!(0x40, false);
                        }
                        
                        0x78...0x7F => {//BIT 7
                            testBit!(0x40, true);
                            testBit!(0x80, false);
                        }


                        _ => {}
                    }
                }

                0x80...0xBF => { //RES
                    macro_rules! testResetBit {
                        ($initialVal: expr, $result: expr)=>  ({
                            storeValue!($initialVal, i);
                            executeInstruction!();

                            testValue!($result, i);

                        })
                    }
                    match i {

                        0x80...0x87 => {//RES 0
                            testResetBit!(0xFF, 0xFE);
                        }

                        0x88...0x8F => {//RES 1
                            testResetBit!(0xF3, 0xF1);
                        }
                        
                        0x90...0x97 => {//RES 2
                            testResetBit!(0x44, 0x40);
                        }
                        
                        0x98...0x9F => {//RES 3
                            testResetBit!(0x4C, 0x44);
                        }

                        0xA0...0xA7 => {//RES 4
                            testResetBit!(0x1C, 0xC);
                        }

                        0xA8...0xAF => {//RES 5
                            testResetBit!(0x2C, 0xC);
                        }
                        
                        0xB0...0xB7 => {//RES 6
                            testResetBit!(0x4C, 0xC);
                        }
                        
                        0xB8...0xBF => {//RES 7
                            testResetBit!(0xFC, 0x7C);
                        }


                        _ => {}
                    }
                }
                
                0xC0...0xFF => { //SET
                    macro_rules! testSetBit {
                        ($initialVal: expr, $result: expr)=>  ({
                            storeValue!($initialVal, i);
                            executeInstruction!();

                            testValue!($result, i);

                        })
                    }
                    match i {

                        0xC0...0xC7 => {//SET 0
                            testSetBit!(0, 1);
                        }

                        0xC8...0xCF => {//SET 1
                            testSetBit!(4, 6);
                        }
                        
                        0xD0...0xD7 => {//SET 2
                            testSetBit!(4, 4);
                        }
                        
                        0xD8...0xDF => {//SET 3
                            testSetBit!(0x7, 0xF);
                        }

                        0xE0...0xE7 => {//SET 4
                            testSetBit!(0, 0x10);
                        }

                        0xE8...0xEF => {//SET 5
                            testSetBit!(0, 0x20);
                        }
                        
                        0xF0...0xF7 => {//SET 6
                            testSetBit!(0, 0x40);
                        }
                        
                        0xF8...0xFF => {//SET 7
                            testSetBit!(0, 0x80);
                        }


                        _ => {}
                    }
                }
                _ => {}
            }
        }

        
    }

}


