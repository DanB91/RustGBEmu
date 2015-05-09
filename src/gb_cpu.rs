/*
 * All Z80 related functions go into this module
 *
 */

use gb_memory::*;

pub struct CPUState {
    pub PC: u16,
    pub SP: u16,
    pub A: u8,
    pub B: u8,
    pub C: u8,
    pub F: u8,
    pub H: u8,
    pub L: u8,
    pub totalCycles: u32, //total cycles since game has been loaded
    pub instructionCycles: u32 //number of cycles in a given instruction

}

impl CPUState {
    pub fn new() -> CPUState {
        CPUState {
            PC: 0,
            SP: 0,
            A: 0,
            B: 0,
            C: 0,
            F: 0,
            H: 0,
            L: 0,
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

/**
  performs the INC X 8-bit instruction

  Examples:
  increment8(cpu, B) // increments the B register and sets appropiate flags
*/

macro_rules! increment8 {
    ($cpu: ident, $reg: ident) => ({
        $cpu.$reg = $cpu.$reg.wrapping_add(1);

        match $cpu.$reg {
            0 => setFlag(Zero, &mut $cpu.F),
            _ => clearFlag(Zero, &mut $cpu.F) 
        };

        clearFlag(Neg, &mut $cpu.F);

        match $cpu.$reg & 0xF {
            0 => setFlag(Half, &mut $cpu.F),
            _ => clearFlag(Half, &mut $cpu.F)
        };
        ($cpu.PC + 1, 4)
    })

}

//returns a tuple of the form (new_PC_value, number_of_cycles_passed)

//NOTE(DanB) the reason I return these values instead of modifying them is because I constantly
//forget to update the PC and cycles passed.  This way, the compiler will force me to do so.
//Perhaps I can find a better way
pub fn executeInstruction(instruction: u8, cpu: &mut CPUState, mem: &mut MemoryState) -> (u16, u32) {

    use self::Flag::*;


    macro_rules! setFlag {
        ($f:expr) => (setFlag($f, &mut cpu.F));
    }

    macro_rules! clearFlag {
        ($f:expr) => (clearFlag($f, &mut cpu.F));
    }

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
            increment8!(cpu,B)
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

            clearFlag!(Zero);
            clearFlag!(Neg);
            clearFlag!(Half);

            match cpu.A & 0x80 {
                0 => clearFlag!(Carry),
                _ => setFlag!(Carry)
            }

            cpu.A = (cpu.A << 1) | (cpu.A >> 7);

            (cpu.PC + 1, 4)
        },

        0x8 => { //LD (a16), SP
            let addr = readWordFromMemory(mem, cpu.PC + 1);

            writeWordToMemory(mem, cpu.SP, addr);

            (cpu.PC + 3, 20)

        },

        0x9 => { //ADD HL, BC
            //NOTE(DanB): Half and Carry flags may or may not be cleared

            clearFlag!(Neg);
            let BC = word(cpu.B, cpu.C) as u32;
            let HL = word(cpu.H, cpu.L) as u32;

            let result = HL.wrapping_add(BC);

            if result & 0x10000 != 0 {
                setFlag!(Carry);
            }
            else {
                clearFlag!(Carry);
            }

            if (HL ^ BC ^ (result & 0xFFFF)) & 0x1000 != 0 {
                setFlag!(Half);
            }
            else {
                clearFlag!(Half);
            }

            cpu.H = hb(result as u16);
            cpu.L = lb(result as u16);

            (cpu.PC + 1, 8)
        },

        0xA => { //LD A, (BC)
            cpu.A = readByteFromMemory(mem, word(cpu.B, cpu.C));
            (cpu.PC + 1, 8)
        },

        0xB => { //DEC BC
            let newVal = word(cpu.B,cpu.C).wrapping_sub(1);
            cpu.B = hb(newVal); cpu.C = lb(newVal);
            (cpu.PC +1, 8)

        },

        0xC => { //INC C
            increment8!(cpu, C)
        }

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

