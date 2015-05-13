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
    pub D: u8,
    pub E: u8,
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
            D: 0,
            E: 0,
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
    *highDest = readByteFromMemory(mem, PC.wrapping_add(2));
    *lowDest = readByteFromMemory(mem, PC.wrapping_add(1));
}

//TODO(DanB): Should I move these macros into executeInstruction()?



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

    macro_rules! isFlagSet {
        ($f:expr) => (isFlagSet($f, cpu.F));
    }

    /*
     * Used for instructions that add a register pair to the HL register pair
     *
     * Args:
     *      srcHigh: high part of register pair to add to HL
     *      srcLow: low part of register pair to add to HL
     *
     * Example:
     *      addToHL!(B, C) //add BC to HL
     *
     */
    macro_rules! addToHL {
        ($srcHigh: ident, $srcLow: ident) => ({

            //NOTE(DanB): Half and Carry flags may or may not be cleared

            clearFlag!(Neg);
            let src = word(cpu.$srcHigh, cpu.$srcLow) as u32;
            let HL = word(cpu.H, cpu.L) as u32;

            let result = HL.wrapping_add(src);

            if result & 0x10000 != 0 {
                setFlag!(Carry);
            }
            else {
                clearFlag!(Carry);
            }

            if (HL ^ src ^ (result & 0xFFFF)) & 0x1000 != 0 {
                setFlag!(Half);
            }
            else {
                clearFlag!(Half);
            }

            cpu.H = hb(result as u16);
            cpu.L = lb(result as u16);

            (cpu.PC.wrapping_add(1), 8)
        })
    }

    /*
     * performs the INC XX, where XX is a 16-bit register pair
     *
     * Examples:
     * increment16(cpu, B, C) // increments  the BC register pair
     */
    macro_rules! increment16 {

        ($regHigh: ident, $regLow: ident) => ({

            let newVal = word(cpu.$regHigh,cpu.$regLow).wrapping_add(1);
            cpu.$regHigh = hb(newVal); cpu.$regLow = lb(newVal);
            (cpu.PC.wrapping_add(1), 8)
        })
    }

    /*
     * performs the INC XX, where XX is a 16-bit register pair
     *
     * Examples:
     * increment16(cpu, B, C) // increments  the BC register pair
     */
    macro_rules! decrement16 {

        ($regHigh: ident, $regLow: ident) => ({

            let newVal = word(cpu.$regHigh,cpu.$regLow).wrapping_sub(1);
            cpu.$regHigh = hb(newVal); cpu.$regLow = lb(newVal);
            (cpu.PC.wrapping_add(1), 8)
        })
    }

    /*
       performs the INC X 8-bit instruction

       Examples:
       increment8(cpu, B) // increments the B register and sets appropiate flags
       */

    macro_rules! increment8 {
        ($reg: ident) => ({
            cpu.$reg = cpu.$reg.wrapping_add(1);

            match cpu.$reg {
                0 => setFlag(Zero, &mut cpu.F),
                _ => clearFlag(Zero, &mut cpu.F) 
            };

            clearFlag(Neg, &mut cpu.F);

            match cpu.$reg & 0xF {
                0 => setFlag(Half, &mut cpu.F),
                _ => clearFlag(Half, &mut cpu.F)
            };
            (cpu.PC.wrapping_add(1), 4)
        })

    }

    /*
       performs the DEC X 8-bit instruction

       Examples:
       decrement8(cpu, B) // decrements the B register and sets appropiate flags
       */
    macro_rules! decrement8 {
        ($reg: ident) => ({
            cpu.$reg = cpu.$reg.wrapping_sub(1);

            match cpu.$reg {
                0 => setFlag(Zero, &mut cpu.F),
                _ => clearFlag(Zero, &mut cpu.F) 
            };

            setFlag(Neg, &mut cpu.F);

            match cpu.$reg & 0xF {
                0xF => setFlag(Half, &mut cpu.F),
                _ => clearFlag(Half, &mut cpu.F)
            };
            (cpu.PC.wrapping_add(1), 4)

        })
    }

    /*
     * Performs the JR series of instructions
     *
     * Args:
     *      condition: whether to do the jump or not.
     *
     * Example:
     *      jumpRelative(isFlagSet!(Carry))
     *
     */
    macro_rules! jumpRelative {
        ($condition: expr) => ({
            //whether or not to do the actual jump
            let offset = if $condition {
                readByteFromMemory(&mem, cpu.PC.wrapping_add(1)) as i8
            }
            else {
                0
            };

            let cycles = if $condition {12} else {8};

            (((cpu.PC as i16).wrapping_add(2).wrapping_add(offset as i16)) as u16, cycles)
        })
    }

    match instruction {
        0x0 => { //NOP
            (cpu.PC.wrapping_add(1),4)
        }, 
        0x1 => { //LD BC, NN
            loadImm16(&mut cpu.B, &mut cpu.C, cpu.PC, &mem);
            (cpu.PC.wrapping_add(3), 12)
        },
        0x2 => { //LD (BC), A
            writeByteToMemory(mem, cpu.A, word(cpu.B, cpu.C));
            (cpu.PC.wrapping_add(1), 8)
        },
        0x3 => { //INC BC
            increment16!(B, C)
        },
        0x4 => { //INC B
            increment8!(B)
        },

        0x5 => { //DEC B
            decrement8!(B)
        },

        0x6 => { //LD B, d8
            cpu.B = readByteFromMemory(&mem, cpu.PC.wrapping_add(1));
            (cpu.PC.wrapping_add(2), 8)
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

            (cpu.PC.wrapping_add(1), 4)
        },

        0x8 => { //LD (a16), SP
            let addr = readWordFromMemory(mem, cpu.PC.wrapping_add(1));

            writeWordToMemory(mem, cpu.SP, addr);

            (cpu.PC.wrapping_add(3), 20)

        },

        0x9 => { //ADD HL, BC
            addToHL!(B,C)
        },

        0xA => { //LD A, (BC)
            cpu.A = readByteFromMemory(mem, word(cpu.B, cpu.C));
            (cpu.PC.wrapping_add(1), 8)
        },

        0xB => { //DEC BC
            decrement16!(B, C)
        },

        0xC => { //INC C
            increment8!(C)
        },

        0xD => { //DEC C
            decrement8!(C)
        },
        
        0xE => { //LD C, d8
            cpu.C = readByteFromMemory(&mem, cpu.PC.wrapping_add(1));
            (cpu.PC.wrapping_add(2), 8)
        },

        0xF => { //RRCA
            clearFlag!(Zero);
            clearFlag!(Neg);
            clearFlag!(Half);

            match cpu.A & 0x1 {
                0 => clearFlag!(Carry),
                _ => setFlag!(Carry)
            }

            cpu.A = (cpu.A >> 1) | (cpu.A << 7);

            (cpu.PC.wrapping_add(1), 4)
        },

        0x10 => { //STOP 0
            //TODO: To be implemented
            debug_assert!(readByteFromMemory(&mem, cpu.PC.wrapping_add(1)) == 0); //next byte should be 0
            (cpu.PC.wrapping_add(2), 4)
        },

        0x11 => { //LD DE, d16
            loadImm16(&mut cpu.D, &mut cpu.E, cpu.PC, &mem);
            (cpu.PC.wrapping_add(3), 12) 
        },

        0x12 => { //LD (BC), A
            writeByteToMemory(mem, cpu.A, word(cpu.D, cpu.E));
            (cpu.PC.wrapping_add(1), 8)
        },

        0x13 => { //INC DE
            increment16!(D, E)
        },
        
        0x14 => { //INC D
            increment8!(D)
        },

        0x15 => { //DEC D
            decrement8!(D)
        },

        0x16 => { //LD D, d8
            cpu.D = readByteFromMemory(&mem, cpu.PC.wrapping_add(1));
            (cpu.PC.wrapping_add(2), 8)
        },

        0x17 => { //RLA
            clearFlag!(Zero);
            clearFlag!(Neg);
            clearFlag!(Half);

            let temp = if isFlagSet!(Carry) {
                cpu.A << 1 | 1
            }
            else {
                cpu.A << 1
            };

            match cpu.A & 0x80 {
                0 => clearFlag!(Carry),
                _ => setFlag!(Carry)
            }

            cpu.A = temp;

            (cpu.PC.wrapping_add(1), 4)

        },

        0x18 => { //JR s8 
            jumpRelative!(true)
        },
        
        0x19 => { //ADD HL, DE
            addToHL!(D,E)
        },

        0x1A => { //LD A, (DE)
            cpu.A = readByteFromMemory(mem, word(cpu.D, cpu.E));
            (cpu.PC.wrapping_add(1), 8)

        },

        0x1B => { //DEC DE
            decrement16!(D, E)
        },

        0x1C => { //INC E
            increment8!(E)
        },

        0x1D => { //DEC E
            decrement8!(E)
        },

        0x1E => { //LD E, d8
            cpu.E = readByteFromMemory(&mem, cpu.PC.wrapping_add(1));
            (cpu.PC.wrapping_add(2), 8)
        },

        0x1F => { //RRA

            clearFlag!(Zero);
            clearFlag!(Neg);
            clearFlag!(Half);

            let temp = if isFlagSet!(Carry) {
                cpu.A >> 1 | 0x80
            }
            else {
                cpu.A >> 1
            };

            match cpu.A & 0x1 {
                0 => clearFlag!(Carry),
                _ => setFlag!(Carry)
            }

            cpu.A = temp;

            (cpu.PC.wrapping_add(1), 4)

        },

        0x20 => { //JR NZ, s8
            jumpRelative!(!isFlagSet!(Zero))
        },

        0x21 => { //LD HL, d16
            loadImm16(&mut cpu.H, &mut cpu.L, cpu.PC, &mem);
            (cpu.PC.wrapping_add(3), 12) 
        },
        
        0x22 => { //LD (HL+), A
            writeByteToMemory(mem, cpu.A, word(cpu.H, cpu.L));
            increment16!(H,L);
            (cpu.PC.wrapping_add(1), 8)
        },
        
        0x23 => { //INC HL
            increment16!(H,L)
        },

        0x24 => { //INC H
            increment8!(H)
        },

        0x25 => { //DEC H
            decrement8!(H)
        },
        
        0x26 => { //LD H, d8
            cpu.H = readByteFromMemory(&mem, cpu.PC.wrapping_add(1));
            (cpu.PC.wrapping_add(2), 8)
        },

        0x27 => { //DAA

            let mut result = cpu.A as u16;

            if !isFlagSet!(Neg) { //if addition was used
                
                if isFlagSet!(Half) || result & 0xF > 0x9 {
                    result = result.wrapping_add(0x6);
                }

                if isFlagSet!(Carry) || result & 0xF0 > 0x90 {
                    result = result.wrapping_add(0x60);
                }

            }
            else { //subtraction used

                if isFlagSet!(Half) {
                    result = result.wrapping_sub(6) & 0xFF;
                }

                if isFlagSet!(Carry) {
                    result = result.wrapping_sub(0x60);
                }

            }

            if result & 100 > 0 {
                setFlag!(Carry);
            }

            clearFlag!(Half);

            if result & 0xFF == 0 {
                setFlag!(Zero);
            }
            else {
                clearFlag!(Zero);
            }

            cpu.A = result as u8;

            (cpu.PC.wrapping_add(1), 4)

        },
        _ => { //will act as a NOP for now
            (cpu.PC.wrapping_add(1), 4)
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

