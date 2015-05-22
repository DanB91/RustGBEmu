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

        assert!(newPC  == (cpu.PC as i16 - 128 + 2) as u16);
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
