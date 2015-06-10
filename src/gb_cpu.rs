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

#[derive(Copy, Clone)]
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

/*
 * Pushes a 16bit value onto the stack
 *
 * Args:
 *      mem: The memory state of the Game Boy which contains the stack
 *      value: the value to push onto the stack
 *      SP: the stack pointer
 *
 */
fn pushOnToStack(mem: &mut MemoryState, value: u16, SP: &mut u16) {
    *SP = SP.wrapping_sub(2);
    writeWordToMemory(mem, value, *SP);
}

/*
 * Pops a 16bit value off the stack
 *
 * Args:
 *      mem: The memory state of the Game Boy which contains the stack
 *      SP: the stack pointer
 *
 * Return: The 16bit value off of the stack
 *
 */
fn popOffOfStack(mem: &MemoryState, SP: &mut u16) -> u16 {
    let ret = readWordFromMemory(mem, *SP);
    *SP = SP.wrapping_add(2);

    ret
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

    macro_rules! isFlagSet {
        ($f:expr) => (isFlagSet($f, cpu.F));
    }

    /*
     * Used for ADD and ADC which are 8-bit additions to register
     *
     * Args:
     *      src:  Amount to add to A
     *      shouldAddCarry: true if ADC instruction, false if ADD instruction
     *
     * Example:
     *      add8Bit!(B, true); //ADC A, B
     */
    macro_rules! add8Bit {
        ($src: expr, $shouldAddCarry: expr) => ({
            let mut sum = (cpu.A as u16).wrapping_add($src as u16); 

            clearFlag!(Neg);

            if $shouldAddCarry {
                sum = sum.wrapping_add(if isFlagSet!(Carry) {1} else {0});
            } 

            if (sum & 0xFF) == 0 {
                setFlag!(Zero);
            }
            else {
                clearFlag!(Zero);
            }

            if (sum & 0x100) != 0 {
                setFlag!(Carry);
            }
            else {
                clearFlag!(Carry);
            }


            if (cpu.A ^ $src ^ (sum & 0xFF) as u8) & 0x10 != 0 {
                setFlag!(Half);
            }
            else {
                clearFlag!(Half);
            }

            cpu.A = (sum & 0xFF) as u8;
        })
    }
    /*
     * Used for SUB and SBC which are 8-bit additions to register
     *
     * Args:
     *      src:  Amount to subtract from A
     *      shouldAddCarry: true if SBC instruction, false if SUB instruction
     *      shouldSaveResult: true if SBC or SUB, false if CP
     *
     * Example:
     *      sub8Bit!(B, true); //SBC A, B
     */
    macro_rules! sub8Bit {
        ($src: expr, $shouldSubCarry: expr, $shouldSaveResult: expr) => ({
            let mut diff = (cpu.A as u16).wrapping_sub($src as u16); 

            setFlag!(Neg);

            if $shouldSubCarry {
                diff = diff.wrapping_sub(if isFlagSet!(Carry) {1} else {0});
            } 

            if (diff & 0xFF) == 0 {
                setFlag!(Zero);
            }
            else {
                clearFlag!(Zero);
            }

            //NOTE(DanB): Setting of flags may be wrong in GB CPU manual.  Using Z80's spec
            if diff > 0xFF {
                setFlag!(Carry);
            }
            else {
                clearFlag!(Carry);
            }


            if (cpu.A ^ $src ^ (diff & 0xFF) as u8) & 0x10 != 0 {
                setFlag!(Half);
            }
            else {
                clearFlag!(Half);
            }

            if $shouldSaveResult {
                cpu.A = (diff & 0xFF) as u8;
            }
        })
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

            //NOTE(DanB): Don't know if Half and Carry flags should or should not be cleared

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
        });

        ($src16: ident) => ({
            //NOTE(DanB): Don't know if Half and Carry flags should or should not be cleared

            clearFlag!(Neg);
            let src = cpu.$src16 as u32;
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
     * increment16(B, C) // increments  the BC register pair
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
            if $condition {
                let offset = readByteFromMemory(&mem, cpu.PC.wrapping_add(1)) as i8;
                (((cpu.PC as i16).wrapping_add(offset as i16)) as u16, 12)
            }
            else {
                (cpu.PC.wrapping_add(2), 8)
            }


        })
    }

    /*
     * Pops word from stack into given register pair
     *
     * Args:
     *      highReg: High byte of register pair
     *      lowReg: Low byte of register pair
     *
     * Example:
     *      pop16!(B,C) //pops 2 bytes into register pair BC
     *
     */
    macro_rules! pop16 {
        ($highReg: ident, $lowReg: ident) => ({
            let word = popOffOfStack(mem, &mut cpu.SP);
            cpu.$highReg = hb(word);
            cpu.$lowReg = lb(word);

            (cpu.PC.wrapping_add(1), 12)
        })
    }

    /*
     * Pushes word onto stack into given register pair
     *
     * Args:
     *      highReg: High byte of register pair
     *      lowReg: Low byte of register pair
     *
     * Example:
     *      push16!(B,C) //pushes 2 bytes from register pair BC
     *
     */
    macro_rules! push16 {
        ($highReg: ident, $lowReg: ident) => ({
            pushOnToStack(mem, word(cpu.$highReg, cpu.$lowReg), &mut cpu.SP);
            (cpu.PC.wrapping_add(1), 16)
        })
    }

    /*
     * Performs the JP series of instructions
     *
     * Args:
     *      condition: whether to do the jump or not.
     *
     * Example:
     *      jumpAbsolute(isFlagSet!(Carry)) //jump if Carry flag is set
     *
     */
    macro_rules! jumpAbsolute {
        ($condition: expr) => ({
            
            //should we perform the jump?
            if $condition {
                (readWordFromMemory(&mem, cpu.PC.wrapping_add(1)), 16)
            }
            else {
                (cpu.PC.wrapping_add(3), 12)
            }
        })
    }

    /*
     * Performs the RET series of instructions
     *
     * Args:
     *      condition: whether to return from the procedure or not.
     *
     * Example:
     *      returnFromProc(isFlagSet!(Carry)) //return from procedure if Carry flag is set
     *
     */
    macro_rules! returnFromProc {
        ($condition: expr) => ({
            if $condition {
                //pop return address off stack
                (popOffOfStack(mem, &mut cpu.SP), 20)
            }
            else {
                (cpu.PC.wrapping_add(1), 8)
            }

        })
    }

    /*
     * Performs the CALL series of instructions
     *
     * Args:
     *      condition: whether to call the procedure or not.
     *
     * Example:
     *      callProc(isFlagSet!(Carry)) //call procedure if Carry flag is set
     *
     */
    macro_rules! callProc {
        ($condition: expr) => ({
            if $condition {
                //save PC
                pushOnToStack(mem, cpu.PC.wrapping_add(3), &mut cpu.SP);

                //jump to procedure
                (readWordFromMemory(mem, cpu.PC.wrapping_add(1)), 24)
            }
            else {
                (cpu.PC.wrapping_add(3), 12)
            }
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

        0x28 => { //JR Z, s8
            jumpRelative!(isFlagSet!(Zero))

        },
        0x29 => { //ADD HL, HL
            addToHL!(H,L)
        },
        
        0x2A => { //LD A, (HL+)
            cpu.A = readByteFromMemory(mem, word(cpu.H, cpu.L));
            increment16!(H,L);
            (cpu.PC.wrapping_add(1), 8)

        },

        0x2B => { //DEC HL
            decrement16!(H,L)
        },

        0x2C => { //INC L
            increment8!(L)
        },

        0x2D => { //DEC L
            decrement8!(L)
        },
        
        0x2E => { //LD L, d8
            cpu.L = readByteFromMemory(&mem, cpu.PC.wrapping_add(1));
            (cpu.PC.wrapping_add(2), 8)
        },

        0x2F => { //CPL
            cpu.A = !cpu.A;
            setFlag!(Neg);
            setFlag!(Half);
            (cpu.PC.wrapping_add(1), 4)

        },
        
        0x30 => { //JR NC, s8
            jumpRelative!(!isFlagSet!(Carry))

        },

        0x31 => { //LD SP, d16
            cpu.SP = readWordFromMemory(&mem, cpu.PC.wrapping_add(1));
            (cpu.PC.wrapping_add(3), 12)
        },
        
        0x32 => { //LD (HL-), A
            writeByteToMemory(mem, cpu.A, word(cpu.H, cpu.L));
            decrement16!(H,L);
            (cpu.PC.wrapping_add(1), 8)
        },

        0x33 => { //INC SP
            cpu.SP = cpu.SP.wrapping_add(1);
            (cpu.PC.wrapping_add(1), 8)
        },

        0x34 => { //INC (HL)

            let val = readByteFromMemory(mem, word(cpu.H, cpu.L)).wrapping_add(1); //incremented value

            match val {
                0 => setFlag(Zero, &mut cpu.F),
                _ => clearFlag(Zero, &mut cpu.F) 
            };

            clearFlag(Neg, &mut cpu.F);

            match val & 0xF {
                0 => setFlag(Half, &mut cpu.F),
                _ => clearFlag(Half, &mut cpu.F)
            };

            writeByteToMemory(mem, val, word(cpu.H, cpu.L));

            (cpu.PC.wrapping_add(1), 12)

        },
        
        0x35 => { //DEC (HL)

            let val = readByteFromMemory(&mem, word(cpu.H, cpu.L)).wrapping_sub(1); //decremented value

            match val {
                0 => setFlag(Zero, &mut cpu.F),
                _ => clearFlag(Zero, &mut cpu.F) 
            };

            setFlag(Neg, &mut cpu.F);

            match val & 0xF {
                0xF => setFlag(Half, &mut cpu.F),
                _ => clearFlag(Half, &mut cpu.F)
            };

            writeByteToMemory(mem, val, word(cpu.H, cpu.L));

            (cpu.PC.wrapping_add(1), 12)

        },

        0x36 => { //LD (HL), d8

            let val = readByteFromMemory(mem, cpu.PC.wrapping_add(1)); //value from memory
            writeByteToMemory(mem, val, word(cpu.H, cpu.L));

            (cpu.PC.wrapping_add(2), 12)

        },

        0x37 => { //SCF
            setFlag!(Carry);
            clearFlag!(Half);
            clearFlag!(Neg);

            (cpu.PC.wrapping_add(1), 4)

        },
        
        0x38 => { //JR C, r8
            jumpRelative!(isFlagSet!(Carry))
        },

        0x39 => { //ADD HL, SP
            addToHL!(SP)
        },
        
        0x3A => { //LD A, (HL-)
            cpu.A = readByteFromMemory(mem, word(cpu.H, cpu.L));
            decrement16!(H,L);
            (cpu.PC.wrapping_add(1), 8)

        },
        
        0x3B => { //DEC SP
            cpu.SP = cpu.SP.wrapping_sub(1);
            (cpu.PC.wrapping_add(1), 8)
        },

        0x3C => { //INC A
            increment8!(A)
        },
        
        0x3D => { //DEC A
            decrement8!(A)
        },
        
        0x3E => { //LD A, d8
            cpu.A = readByteFromMemory(mem, cpu.PC.wrapping_add(1));
            (cpu.PC.wrapping_add(2), 8)
        },
        
        0x3F => { //CCF
            clearFlag!(Half);
            clearFlag!(Neg);

            if isFlagSet!(Carry) {
                clearFlag!(Carry);
            } 
            else {
                setFlag!(Carry);
            }

            (cpu.PC.wrapping_add(1), 4)
        },

        0x40...0x6F | 0x78...0x7F => { //8 bit load instructions except for LD (HL), Reg

            let src = match (instruction & 0xF) % 8 {
                0 => cpu.B,
                1 => cpu.C,
                2 => cpu.D,
                3 => cpu.E,
                4 => cpu.H,
                5 => cpu.L,
                6 => readByteFromMemory(mem, word(cpu.H, cpu.L)),
                7 => cpu.A,
                _ => panic!("Unreachable")
            };

            let dest = match instruction {
                0x40...0x47 => &mut cpu.B,
                0x48...0x4F => &mut cpu.C,
                0x50...0x57 => &mut cpu.D,
                0x58...0x5F => &mut cpu.E,
                0x60...0x67 => &mut cpu.H,
                0x68...0x6F => &mut cpu.L,
                0x78...0x7F => &mut cpu.A,
                _ => panic!("Unreachable")
            };

            *dest = src;

            //instructions that have (HL) in the instruction take 8 cycles as opposed to 4
            if ((instruction & 0xF) % 8) == 6 {
                (cpu.PC.wrapping_add(1), 8)
            }
            else {
                (cpu.PC.wrapping_add(1), 4)
            }
        },

        0x70...0x75 | 0x77 => { //LD (HL), N

            let src = match (instruction & 0xF) % 8 {
                0 => cpu.B,
                1 => cpu.C,
                2 => cpu.D,
                3 => cpu.E,
                4 => cpu.H,
                5 => cpu.L,
                7 => cpu.A,
                _ => panic!("Unreachable")
            };

            writeByteToMemory(mem, src, word(cpu.H, cpu.L));

            (cpu.PC.wrapping_add(1), 8)

        },

        0x76 => { //HALT
            //TODO(DanB): to be implemented....
            (cpu.PC.wrapping_add(1), 4)

        },

        0x80...0xBF => { //ADD, ADC, SUB, SBC, AND, XOR, OR and CP instructions, where destination is register A
            let src = match (instruction & 0xF) % 8 {
                0 => cpu.B,
                1 => cpu.C,
                2 => cpu.D,
                3 => cpu.E,
                4 => cpu.H,
                5 => cpu.L,
                6 => readByteFromMemory(mem, word(cpu.H, cpu.L)),
                7 => cpu.A,
                _ => panic!("Unreachable")
            };


            match instruction {
                0x80...0x87 => { //ADD A, N
                    add8Bit!(src, false);
                }

                0x88...0x8F => { //ADC A, N
                    add8Bit!(src, true);
                }
                
                0x90...0x97 => { //SUB N
                    sub8Bit!(src, false, true);
                }
                
                0x98...0x9F => { //SBC N
                    sub8Bit!(src, true, true);
                }

                0xA0...0xA7 => { //AND N
                    cpu.A &= src;
                    
                    setFlag!(Half);
                    clearFlag!(Neg);
                    clearFlag!(Carry);

                    if cpu.A == 0 {
                        setFlag!(Zero);
                    }
                    else {
                        clearFlag!(Zero);
                    }

                }

                0xA8...0xAF => { //XOR N
                    cpu.A ^= src;
                    
                    clearFlag!(Half);
                    clearFlag!(Neg);
                    clearFlag!(Carry);

                    if cpu.A == 0 {
                        setFlag!(Zero);
                    }
                    else {
                        clearFlag!(Zero);
                    }
                }

                0xB0...0xB7 => { //OR N

                    cpu.A |= src;
                    
                    clearFlag!(Half);
                    clearFlag!(Neg);
                    clearFlag!(Carry);

                    if cpu.A == 0 {
                        setFlag!(Zero);
                    }
                    else {
                        clearFlag!(Zero);
                    }
                }

                0xB8...0xBF => { //CP N
                    sub8Bit!(src, false, false);
                }
                
                _ => panic!("Unreachable")
            }

            if ((instruction & 0xF) % 8) == 6 {
                (cpu.PC.wrapping_add(1), 8)  //if operating from (HL), inst takes 8 cycles
            }
            else {
                (cpu.PC.wrapping_add(1), 4) //operating from register takes 4 cycles
            }


        },

        0xC0 => { //RET NZ
            returnFromProc!(!isFlagSet!(Zero))
        },

        0xC1 => { //POP BC
            pop16!(B,C)

        },

        0xC2 => { //JP NZ, a16
            jumpAbsolute!(!isFlagSet!(Zero))
        }

        0xC3 => { //JP a16
            jumpAbsolute!(true)
        },

        0xC4 => { //CALL NZ, a16
            callProc!(!isFlagSet!(Zero))
        },

        0xC5 => { //PUSH BC
            push16!(B,C)
        }

        0xC8 => { //RET Z 
            returnFromProc!(isFlagSet!(Zero))
        },

        0xC9 => { //RET
            (popOffOfStack(mem, &mut cpu.SP), 16)
        }

        0xCA => { //JP Z, a16
            jumpAbsolute!(isFlagSet!(Zero))
        },

        0xCC => { //CALL Z, a16
            callProc!(isFlagSet!(Zero))
        },
        
        0xCD => { //CALL a16
            callProc!(true)
        },

        0xD0 => { //RET NC
            returnFromProc!(!isFlagSet!(Carry))
        },

        0xD1 => { //POP DE
            pop16!(D,E)
        },

        0xD2 => { //JP NC, a16
            jumpAbsolute!(!isFlagSet!(Carry))
        },
        
        0xD4 => { //CALL NC, a16
            callProc!(!isFlagSet!(Carry))
        },
        
        0xD5 => { //PUSH DE
            push16!(D,E)
        },
        
        0xD8 => { //RET C
            returnFromProc!(isFlagSet!(Carry))
        }

        0xDA => { // JP C, a16
            jumpAbsolute!(isFlagSet!(Carry))
        },

        0xDC => { //CALL C, a16
            callProc!(isFlagSet!(Carry))
        },

        0xE1 => { //POP HL
            pop16!(H,L)
        }

        0xE5 => { //PUSH HL
            push16!(H,L)
        }

        0xF1 => { //POP AF
            pop16!(A,F)
        }

        0xF5 => { //PUSH AF
            push16!(A,F)
        }
        _ => { //TODO:will act as a NOP for now
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

