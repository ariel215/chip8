use ndarray::prelude::*;

type Addr = u16;
type Reg = u8;
type Display = Array2<bool>;


pub mod frontend;
pub mod errors;
pub mod instructions;
pub mod emulator;


pub struct Emulator{
    clock_speed: u64, // Cycles per second,
    memory: Memory,
    registers: Registers,
    frontend: Box<dyn frontend::Chip8Frontend>,
    mode: EmulatorMode
}

/// What kind of display to use for the emulator
pub enum DisplayMode{
    Windowed,
    // Todo: add TUI frontend
}

pub(crate) enum EmulatorMode {
    Running,
    Paused, 
}


#[derive(Debug, PartialEq, Eq)]
// Todo: turn these docs into attributes for a proc macro
// ideally could derive: instruction->mnemonic, mnemonic-> instruction,
// instruction -> u16, u16 -> instruction
// all from attributes 
pub enum Instruction{
    /// CLS
    /// 0x00e0
    /// Clear screen
    ClearScreen,
    /// RET
    /// 0x00ee
    /// Return
    Ret, // Return 
    /// NOP
    /// 0x0000 or anything that isn't a valid instruction
    Nop,
    /// JP imm12
    /// 0x1NNN
    /// Jump to addr
    Jump(Addr),
    /// CALL imm12
    /// 0x2NNN
    ///  call function at addr
    Call(Addr),
    /// SE Vx imm8
    /// 0x3XNN
    /// Skip next instruction if  *reg == imm8
    SkipEqImm(Reg,u8),
    /// SNE Vx imm8
    /// 0x4xnn
    /// Skip next instruction if *reg != imm8
    SkipNeImm(Reg, u8),
    /// SE Vx Vy
    /// 0x5XY0
    /// Skip next instruction if *Vx == *Vy
    SkipEqReg(Reg, Reg),
    /// LD Vx imm8
    /// 0x6XNN
    /// Set *Vx to imm8
    SetImm(Reg, u8),
    /// ADD Vx imm8
    /// 0x7XNN
    /// *Vx += imm
    AddImm(Reg, u8),
    /// LD Vx Vy 
    /// 0x8XY0
    SetReg(Reg, Reg), // Set *Vx to *Vy
    /// OR Vx Vy
    /// 0x8XY1
    OrReg(Reg, Reg), // *Vx |= *Vy
    /// AND Vx Vy
    /// 0x8XY2
    AndReg(Reg, Reg), // *Vx &= *Vy
    /// XOR Vx Vy
    /// 0x8XY3
    XorReg(Reg, Reg), // *Vx ^= Vy
    /// ADD Vx Vy
    /// 0x8XY4
    AddReg(Reg, Reg), // *Vx += *Vy
    /// SUB Vx Vy
    /// 0x8XY5
    SubReg(Reg, Reg), // *Vx -= *Vy; set VF to 1 if the subtraction succeds
    /// RSH Vx Vy
    /// 0x8XY6
    Rsh(Reg), // *Vx >>= 1; store least significant bit in VF
    /// SUBN Vx Vy
    /// 0x8XY7
    SubFrom(Reg, Reg), // Vx = Vy - Vx; set VF to 1 if the subtraction succeeds
    /// LSH Vx Vy
    /// 0x8XYE
    Lsh(Reg), // *Vx <<= 1; store most significant bit in VF
    /// SKNV Vx Vy
    /// 0x9XY0
    SkipNeReg(Reg, Reg), // Skip next instruction if Vx != Vy
    /// LD I imm12
    /// 0xANNN
    SetMemPtr(u16), // Sets the I register to imm
    /// JP V0 imm12
    /// 0xBNNN
    JumpOffset(u16), // Jump to V0 + imm
    /// RND Vx imm8
    /// 0xCXNN
    Rand(Reg, u8), //Set Vx to rand() & imm
    /// DRW Vx Vy imm4
    /// 0xDXYN
    /// Draw N-byte sprite at (Vx,Vy) 
    /// Successive bytes are drawn one below the next
    /// Note that the chip8 display is indexed like (column, row)
    Draw(Reg,Reg, u8), 
    /// SKP Vx
    /// 0xEX9E
    SkipKeyPressed(Reg), // Skip next instruction if the key in Vx is pressed
    /// SKNP Vx
    /// 0xEXA1
    SkipKeyNotPressed(Reg), // Skip next instruction if the key in Vx is *not* pressed
    /// LD Vx
    /// 0xFX07
    GetDelay(Reg), // Set Vx to the value of the delay timer
    /// LD Vx, K
    /// 0xFX0A
    WaitForKey(Reg), // Block until key pressed, then store the key pressed in Vx
    /// MOVD Vx
    /// 0xFX15
    SetDelay(Reg), // Set delay timer to *Vx
    /// MOVS Vx
    /// 0xFX18
    SetSound(Reg), // Set sound timer to *Vx
    /// ADD I Vx
    /// 0xFX1E
    AddMemPtr(Reg), // *I += *Vx
    /// LD F Vx
    /// 0xFX29
    SetChar(Reg), // *I = sprites[SPRITE_LEN * *Vx]
    /// LD B Vx
    /// 0xFX33
    BCD(Reg), // Store the binary-coded decimal representation of Vx at I..=I+2
    /// RDMP Vx
    /// 0xFX55
    RegDump(Reg), // Store registers V0..Vx in memory, starting at I
    /// RLOAD Vx
    /// 0xFX55
    RegLoad(Reg), // Fill registers V0..Vx from memory, starting at I
}

#[derive(Debug)]
pub struct Memory{
    /// Random access memory
    ram: [u8;4096],
    /// Graphic display
    display: Display,
    /// Key array
    keys: [bool; 16],
    // call stack
    stack: Vec<usize>
}

#[derive(Debug)]
pub struct Registers{
    /// General purpose registers
    vn: [u8;16], 
    /// Delay register - counts down at 60 hz
    delay: u8,
    /// Sound register - counts down at 60 hz
    /// CHIP-8 plays a tone if it's set
    sound: u8,
    /// Program counter
    pc: usize,
    /// Stack pointer
    sp: usize,
    /// RAM pointer
    i: usize,
    /// When set, stores the register to store the next keypress in
    key_flag: Option<usize>
}


const CHAR_SPRITES: [u8;16*5] = [
    0xf0, 0x90, 0x90, 0x90, 0xf0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xf0, 0x10, 0xf0, 0x80, 0xf0, // 2
    0xf0, 0x10, 0xf0, 0x10, 0xf0, // 3
    0x90, 0x90, 0xf0, 0x10, 0x10, // 4
    0xf0, 0x80, 0xf0, 0x10, 0xf0, // 5
    0xf0, 0x80, 0xf0, 0x90, 0xf0, // 6
    0xf0, 0x10, 0x20, 0x40, 0x40, // 7
    0xf0, 0x90, 0xf0, 0x90, 0xf0, // 8
    0xf0, 0x90, 0xf0, 0x10, 0xf0, // 9
    0xf0, 0x90, 0xf0, 0x90, 0x90, // A
    0xe0, 0x90, 0xe0, 0x90, 0xe0, // B
    0xf0, 0x80, 0x80, 0x80, 0xf0, // C
    0xe0, 0x90, 0x90, 0x90, 0xe0, // D
    0xf0, 0x80, 0xf0, 0x80, 0xf0, // E
    0xf0, 0x80, 0xf0, 0x80, 0x80  // F
];


const DISPLAY_COLUMNS: usize = 64;
const DISPLAY_ROWS: usize = 32;

