use gb_memory::*;

pub struct CPUState {
    pub PC: u16,
    pub A: u8,
    pub B: u8,
    pub C: u8,
    pub F: u8,
    pub totalCycles: u32, //total cycles since game has been loaded
    pub instructionCycles: u32 //number of cycles in a given instruction

}

impl CPUState {
    pub fn new() -> CPUState {
        CPUState {
            PC: 0,
            A: 0,
            B: 0,
            C: 0,
            F: 0,
            totalCycles: 0,
            instructionCycles: 0
        }
    }
}

pub enum Flag {
    Zero = 0x80,
    Neg = 0x40,
    Half = 0x20,
    Carry = 0x10
}


pub fn setFlag(flag: Flag, F: &mut u8) {
    *F |= flag as u8;
}

pub fn clearFlag(flag: Flag, F: &mut u8) {
    *F &= !(flag as u8);
}

pub fn isFlagSet(flag: Flag, F: u8) -> bool {
    flag as u8 & F != 0
}


//return number of bytes to increment PC by
fn loadImm16(highDest: &mut u8, lowDest: &mut u8, PC: u16, mem: &MemoryState){
    *highDest = readByteFromMemory(mem, PC+2);
    *lowDest = readByteFromMemory(mem, PC+1);
}



//returns a tuple of the form (new_PC_value, number_of_cycles_passed)

//NOTE(DanB) the reason I return these values instead of modifying them is because I constantly
//forget to update the PC and cycles passed.  This way, the compiler will force me to do so.
//Perhaps I can find a better way
pub fn executeInstruction(instruction: u8, cpu: &mut CPUState, mem: &mut MemoryState) -> (u16, u32) {

    use self::Flag::*;
    match instruction {
        0x0 => { //NOP
            (cpu.PC + 1,4)
        }, 
        0x1 => { //LD BC, NN
            loadImm16(&mut cpu.B, &mut cpu.C, cpu.PC, &mem);
            (cpu.PC + 3, 12)
        },
        0x2 => { //LD (BC), A
            writeByteToMemory(mem, cpu.A, word(cpu.B, cpu.C));
            (cpu.PC + 1, 8)
        },
        0x3 => { //INC BC
            let newVal = word(cpu.B,cpu.C).wrapping_add(1);
            cpu.B = hb(newVal); cpu.C = lb(newVal);
            (cpu.PC +1, 8)
        },
        0x4 => { //INC B
            cpu.B = cpu.B.wrapping_add(1);

            match cpu.B {
                0 => setFlag(Zero, &mut cpu.F),
                _ => clearFlag(Zero, &mut cpu.F) 
            };

            clearFlag(Neg, &mut cpu.F);

            match cpu.B & 0xF {
                0 => setFlag(Half, &mut cpu.F),
                _ => clearFlag(Half, &mut cpu.F)
            };
            (cpu.PC + 1, 4)
        },

        0x5 => { //DEC B
            cpu.B = cpu.B.wrapping_sub(1);

            match cpu.B {
                0 => setFlag(Zero, &mut cpu.F),
                _ => clearFlag(Zero, &mut cpu.F) 
            };

            setFlag(Neg, &mut cpu.F);

            match cpu.B & 0xF {
                0xF => setFlag(Half, &mut cpu.F),
                _ => clearFlag(Half, &mut cpu.F)
            };
            (cpu.PC + 1, 4)
        },

        0x6 => { //LD B, d8
            cpu.B = readByteFromMemory(&mem, cpu.PC + 1);
            (cpu.PC + 2, 8)
        },

        0x7 => { //RLCA

            clearFlag(Zero, &mut cpu.F);
            clearFlag(Neg, &mut cpu.F);
            clearFlag(Half, &mut cpu.F);

            match cpu.A & 0x80 {
                0 => clearFlag(Carry, &mut cpu.F),
                _ => setFlag(Carry, &mut cpu.F)
            }

            cpu.A = (cpu.A << 1) | (cpu.A >> 7);

            (cpu.PC + 1, 4)
        },

        _ => { //will act as a NOP for now
            (cpu.PC + 1, 4)
        },
    }
}

pub fn step(cpu: &mut CPUState, mem: &mut MemoryState) {
    let instructionToExecute = readByteFromMemory(&mem, cpu.PC);
    
    if cpu.PC > 0xFF {
        mem.inBios = false;
    }

    let (newPC, cyclesTaken) = executeInstruction(instructionToExecute, cpu, mem); 
    cpu.PC = newPC;
    cpu.instructionCycles = cyclesTaken;
    cpu.totalCycles += cyclesTaken;
}

#[cfg(test)]
mod tests {

    use super::*;
    use gb_memory::*;
    use super::Flag::*;
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

        let mut cpu = testingCPU();
        let mut mem = tetrisMemoryState();

        //test half carry and zero set
        cpu.B = 0xFF;

        let (newPC, cyclesTaken) = executeInstruction(4, &mut cpu, &mut mem);

        assert!(newPC == cpu.PC + 1);
        assert!(cyclesTaken == 4);

        assert!(cpu.B == 0);
        
        assert!(isFlagSet(Half, cpu.F));
        assert!(isFlagSet(Zero, cpu.F));
        assert!(!isFlagSet(Neg, cpu.F));

        //test half carry and zero clear
        
        cpu = testingCPU();
        mem = tetrisMemoryState();
        cpu.B = 0x1;

        let (newPC, cyclesTaken) = executeInstruction(4, &mut cpu, &mut mem);

        assert!(newPC == cpu.PC + 1);
        assert!(cyclesTaken == 4);

        assert!(cpu.B == 2);
        
        assert!(!isFlagSet(Half, cpu.F));
        assert!(!isFlagSet(Zero, cpu.F));
        assert!(!isFlagSet(Neg, cpu.F));

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

        writeWordToMemory(&mut mem, 0xAA06, cpu.PC);

        step(&mut cpu, &mut mem);

        assert!(cpu.B == 0xAA);
        assert!(cpu.instructionCycles == 8);
        assert!(cpu.PC == oldPC + 2);
    }
}
