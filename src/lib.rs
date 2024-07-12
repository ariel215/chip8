use std::{thread, time::{self, Duration}};
use raylib;
use rand;

type Addr = u16;
type Reg = u8;

pub mod graphics;


#[derive(Debug, PartialEq, Eq)]
pub enum Instruction{
    ClearScreen, // Clear screen
    Ret, // Return 
    Nop, // Noop
    Jump(Addr), // Jump to addr
    Call(Addr), // call function at addr
    SkipEqImm(Reg,u8), // Skip next instruction if  *reg == imm8
    SkipNeImm(Reg, u8),  // Skip next instruction if *reg != imm8
    SkipEqReg(Reg, Reg), // Skip next instruction if *Vx == *Vy
    SetImm(Reg, u8), // Set *Vx to imm8
    SetReg(Reg, Reg), // Set *Vx to *Vy
    AddImm(Reg, u8), // *Vx += imm
    OrReg(Reg, Reg), // *Vx |= *Vy
    AndReg(Reg, Reg), // *Vx &= *Vy
    XorReg(Reg, Reg), // *Vx ^= Vy
    AddReg(Reg, Reg), // *Vx += *Vy
    SubReg(Reg, Reg), // *Vx -= *Vy; set VF to 1 if the subtraction succeds
    Rsh(Reg), // *Vx >>= 1; store least significant bit in VF
    SubFrom(Reg, Reg), // Vx = Vy - Vx; set VF to 1 if the subtraction succeeds
    Lsh(Reg), // *Vx <<= 1; store most significant bit in VF
    SkipNeReg(Reg, Reg), // Skip next instruction if Vx != Vy
    SetMemPtr(u16), // Sets the I register to imm
    JumpOffset(u16), // Jump to V0 + imm
    Rand(Reg, u8), //Set Vx to rand() & imm
    Draw(Reg,Reg, u8), // Draw N-byte sprite at (Vx,Vy)
    SkipKey(Reg), // Skip next instruction if the key in Vx is pressed
    SkipNoKey(Reg), // Skip next instruction if the key in Vx is *not* pressed
    GetDelay(Reg), // Set Vx to the value of the delay timer
    WaitForKey(Reg), // Block until key pressed, then store the key pressed in Vx
    SetDelay(Reg), // Set delay timer to *Vx
    SetSound(Reg), // Set sound timer to *Vx
    AddMemPtr(Reg), // *I += *Vx
    SetChar(Reg), // *I = sprites[SPRITE_LEN * *Vx] 
    BCD(Reg), // Store the binary-coded decimal representation of Vx at I..=I+2
    RegDump(Reg), // Store registers V0..Vx in memory, starting at I
    RegLoad(Reg), // Fill registers V0..Vx from memory, starting at I
}


const CHAR_SPRITES: [u8;16*5] = [
    0xf0, 0x90, 0x80, 0x90, 0xf0, // 0
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


const DISPLAY_ROWS: usize = 64;
const DISPLAY_COLUMNS: usize = 32;

#[derive(Debug)]
pub struct Memory{
    /// Random access memory
    ram: [u8;4096],
    /// Graphic display
    display: [bool; DISPLAY_ROWS * DISPLAY_COLUMNS],
    /// Key array
    keys: [bool; 16]
}

#[macro_export]
macro_rules! index {
    ($row: expr, $col: expr) => {
        ($row * crate::DISPLAY_COLUMNS + $col) as usize
    };
}


#[test]
fn test_index (){
    assert_eq!(index!(0,2),2);
    assert_eq!(index!(1,0),32)
}

impl Default for Memory {
    fn default() -> Self {
        let mut mem = Self { ram: [0;4096], display: [false ;64 * 32], keys: Default::default() };
        mem.ram[0..CHAR_SPRITES.len()].copy_from_slice(&CHAR_SPRITES);
        mem
    }
}


#[inline]
fn get_bit(char: u8, index: usize) -> bool{
    let mask = 1 << 7-index;
    let masked_char = char & mask;
    (masked_char >> index) != 0
}

#[test]
fn test_bytes(){
    let test:u8 = 0b01001011;
    assert_eq!(get_bit(test, 0), false);
    assert_eq!(get_bit(test, 1), true);
    assert_eq!(get_bit(test,2), false);
    assert_eq!(get_bit(test,3), false);
}

impl Memory{
    pub fn load_rom(&mut self, rom: &[u8]){
        self.ram[0x200..0x200 + rom.len()].copy_from_slice(rom);
    }
    
    fn set_row(&mut self, x: usize, y: usize, row: u8){
        for i in 0..8 {
            self.display[index!(x, y+i)] = get_bit(row, i)
        }
    }

    pub fn print_display(&self) -> String {
        let mut display_str = String::new();
        for row in 0..DISPLAY_ROWS {
            for col in 0..DISPLAY_COLUMNS{
                if self.display[index!(row,col)]{
                    display_str += "*"
                } else { display_str += " "}
            }
            display_str += "\n"
        }
        display_str
    }

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
    /// when Some: block until key press, then store key 
    /// in the register and reset
    key_flag: Option<usize>
}

impl Default for Registers {
    fn default() -> Self {
        Self { 
            vn: Default::default(), 
            delay: Default::default(), 
            sound: Default::default(),
            // Programs start as 0x200 and grow up
            pc: 0x200,
            // Call stack starts at 0x1ff and grows down 
            sp: 0x1ff,
            i: Default::default(), 
            key_flag: Default::default() }
    }
}



pub struct Emulator{
    clock_speed: u64, // Cycles per second,
    memory: Memory,
    registers: Registers,
    frontend: Box<dyn graphics::Chip8Frontend>
}


impl Emulator{
    pub fn windowed()->Self{
        Self{
            clock_speed: 500,
            memory: Memory::default(),
            registers: Registers::default(),
            frontend: Box::new(graphics::RaylibDisplay::new())
        }
    }

    pub fn clock_speed(&mut self, speed: u64) -> &mut Self{
        self.clock_speed = speed;
        self
    }

    pub fn load_rom(&mut self, rom: &[u8])->&mut Self {
        self.memory.load_rom(rom);
        self
    }

    pub fn step(&mut self) -> bool{
        do_instruction(&mut self.memory, &mut self.registers);
        self.frontend.update(&self.memory.display)
    }

    pub fn run(&mut self){
        let cycle_length = Duration::from_millis(1000 / self.clock_speed);
        let frame_length = Duration::from_millis(1000/60);
        loop {
            let mut frame_elapsed = Duration::ZERO;
            while frame_elapsed < frame_length{
                self.memory.keys = [false; 16];
                let tic = time::Instant::now();
                if let Some(key) = self.frontend.get_input(){
                    self.memory.keys[key as usize] = true;
                    self.registers.key_flag = None;
                } else {
                    if self.registers.key_flag.is_some(){
                        continue;
                    }
                }
                do_instruction(&mut self.memory, &mut self.registers);
                let toc = time::Instant::now();
                if toc - tic < cycle_length{
                    thread::sleep(cycle_length - (toc-tic))
                }
                frame_elapsed += time::Instant::now() - tic;
            }
            if self.registers.delay > 0{
                self.registers.delay -= 1;
            }
            if self.registers.sound > 0 {
                self.registers.sound -= 1;
            }
            if self.frontend.update(&self.memory.display){
                break;
            }
        }
    }
}

// First register argument
macro_rules! X {
    ($opcode: expr) => {{
        let reg = ($opcode & 0x0F00) >> 8;
        assert!(reg < u8::MAX.into());
        reg as u8
    }
    };
}

// Second register argument
macro_rules! Y {
    ($opcode: expr) => {
{        let reg = ($opcode & 0x00F0) >> 4;
        assert!(reg < u8::MAX.into());
        reg as u8
}    };
}

// 4-bit immediate
macro_rules! N {
    ($opcode: expr) => {
        ($opcode & 0x000F) as u8
    }
}

// 8-bit immediate
macro_rules! NN {
    ($opcode: expr) => {
        ($opcode & 0x00FF) as u8
    };
}

// 12-bit immediate
macro_rules! NNN {
    ($opcode: expr) => {
        $opcode & 0x0FFF
    };
}

macro_rules! XNN {
    ($reg: expr, $imm: expr) => {
        ($reg as u16) << 8 | $imm as u16
    };
}

macro_rules! XY {
    ($r1: expr, $r2: expr) => {
        ($r1 as u16) << 8 | ($r2 << 4) as u16
    };
}

impl From<Instruction> for u16 {
    fn from(value: Instruction) -> Self {
        match value {
            Instruction::Nop => 0x0000,
            Instruction::ClearScreen => 0x00E0,
            Instruction::Ret => 0x00EE,
            Instruction::Jump(v) => 0x1000 | v,
            Instruction::Call(v) => 0x2000 | v,
            Instruction::SkipEqImm(reg, imm) => 0x3000 | XNN!(reg, imm),
            Instruction::SkipNeImm(reg, imm ) =>  0x4000 | XNN!(reg, imm),
            Instruction::SkipEqReg(r1, r2 ) => 0x5000 | XY!(r1, r2),
            Instruction::SetImm(reg, imm) => 0x6000 | XNN!(reg, imm),
            Instruction::AddImm(reg, imm) => 0x7000 | XNN!(reg, imm),
            Instruction::SetReg(r1,r2) => 0x8000 | XY!(r1, r2) | 0,
            Instruction::OrReg(r1,r2 ) => 0x8000 | XY!(r1, r2) | 1,
            Instruction::AndReg(r1,r2 ) => 0x8000 | XY!(r1, r2) | 2,
            Instruction::XorReg(r1,r2 ) => 0x8000 | XY!(r1, r2) | 3,
            Instruction::AddReg(r1,r2 ) => 0x8000 | XY!(r1, r2) | 4,
            Instruction::SubReg(r1,r2 ) => 0x8000 | XY!(r1, r2) | 5,
            Instruction::Rsh(r1 ) => 0x8000 | XY!(r1, 0) | 6,
            Instruction::SubFrom(r1,r2 ) => 0x8000 | XY!(r1, r2) | 7,
            Instruction::Lsh(r1 ) => 0x8000 | XY!(r1, 0) | 0xe,
            Instruction::SkipNeReg(r1,r2 )=> 0x9000 | XY!(r1, r2),
            Instruction::SetMemPtr(imm) => 0xA000 | imm,
            Instruction::JumpOffset(imm) => 0xB000 | imm,
            Instruction::Rand(reg,imm ) => 0xC000 | XNN!(reg, imm),
            Instruction::Draw(x,y ,n) => 0xD000 | XY!(x,y) | n as u16,
            Instruction::SkipKey(reg) => 0xE09E | XY!(reg,0),
            Instruction::SkipNoKey(reg) => 0xE0A1 | XY!(reg,0),
            Instruction::GetDelay(reg) => 0xF007 | XY!(reg,0),
            Instruction::WaitForKey(reg) => 0xF009 | XY!(reg,0),
            Instruction::SetDelay(reg) => 0xF015 | XY!(reg,0),
            Instruction::SetSound(reg) => 0xF018 | XY!(reg,0),
            Instruction::AddMemPtr(reg) => 0xF01E | XY!(reg,0),
            Instruction::SetChar(reg) => 0xF029 | XY!(reg,0),
            Instruction::BCD(reg) => 0xF033 | XY!(reg,0),
            Instruction::RegDump(reg) => 0xF055 | XY!(reg,0),
            Instruction::RegLoad(reg) => 0xF065 | XY!(reg,0)
        }
    }
}

impl From<u16> for Instruction {
    fn from(opcode: u16) -> Self {
        match opcode {
            0x00E0 => Self::ClearScreen,
            0x00EE => Self::Ret,
            0x1000..=0x1fff => {
                Self::Jump(NNN!(opcode))
            },
            0x2000..=0x2fff => {
                Self::Call(NNN!(opcode))
            },
            0x3000..=0x3fff => {
                Self::SkipEqImm(X!(opcode), NN!(opcode))
            },
            0x4000..=0x4fff => {
                Self::SkipNeImm(X!(opcode), NN!(opcode))
            },
            0x5000..=0x5ff0 => {
                Self::SkipEqReg(X!(opcode), Y!(opcode))
            },
            0x6000..=0x6fff => {
                Self::SetImm(X!(opcode), NN!(opcode))
            },
            0x7000..=0x7fff => {
                Self::AddImm(X!(opcode), NN!(opcode))
            },
            0x8000..=0x8fff => {
                match opcode & 0x000f{
                    0x0000 => Self::SetReg(X!(opcode), Y!(opcode)),
                    0x0001 => Self::OrReg(X!(opcode), Y!(opcode)),
                    0x0002 => Self::AndReg(X!(opcode), Y!(opcode)),
                    0x0003 => Self::XorReg(X!(opcode), Y!(opcode)),
                    0x0004 => Self::AddReg(X!(opcode), Y!(opcode)),
                    0x0005 => Self::SubReg(X!(opcode), Y!(opcode)),
                    0x0006 => Self::Rsh(X!(opcode)),
                    0x0007 => Self::SubFrom(X!(opcode), Y!(opcode)),
                    0x000E => Self::Lsh(X!(opcode)),
                    _ => Self::Nop
                }
            },
            0x9000..=0x9ff0 => Self::SkipNeReg(X!(opcode), Y!(opcode)),
            0xA000..=0xAfff => Self::SetMemPtr(NNN!(opcode)),
            0xB000..=0xBfff => Self::JumpOffset(NNN!(opcode)),
            0xC000..=0xCfff => Self::Rand(X!(opcode), NN!(opcode)),
            0xD000..=0xDfff => Self::Draw(X!(opcode), Y!(opcode), N!(opcode)),
            0xE000..=0xEfff => {
                let lower = NN!(opcode);
                match lower {
                    0x9E => Self::SkipKey(X!(opcode)),
                    0xA1 => Self::SkipNoKey(X!(opcode)),
                    _ => Self::Nop
                }
            },
            0xF000..=0xFfff => {
                let lower = NN!(opcode);
                match lower {
                    0x07 => Self::GetDelay(X!(opcode)),
                    0x0A => Self::WaitForKey(X!(opcode)),
                    0x15 => Self::SetDelay(X!(opcode)),
                    0x18 => Self::SetSound(X!(opcode)),
                    0x1E => Self::AddMemPtr(X!(opcode)),
                    0x29 => Self::SetChar(X!(opcode)),
                    0x33 => Self::BCD(X!(opcode)),
                    0x55 => Self::RegDump(X!(opcode)),
                    0x65 => Self::RegLoad(X!(opcode)),
                    _ => Self::Nop
                }
            }
            _ => Self::Nop
        }
    }
}

/// Get the current instruction from memory
pub(crate) fn get_instruction(memory: &Memory, registers: &Registers) -> Instruction{
    let upper = memory.ram[registers.pc];
    let lower = memory.ram[registers.pc + 1];
    (((upper as u16) << 8) | (lower as u16)).into()
}


fn add_with_overflow(a: u8, b: u8)-> (u8, bool) {
    let larger = {if a > b {a} else {b}};
    let smaller = if larger == a {b} else {a};
    if let Some(v) = larger.checked_add(smaller){
        (v, false)
    } else {
        (smaller - (u8::MAX - larger) - 1, true)
    }
}

#[test]
fn test_overflow_add(){
    assert_eq!(add_with_overflow(255, 1), (0, true));
    assert_eq!(add_with_overflow(255, 255), (254, true));
    assert_eq!(add_with_overflow(128, 127), (255, false));
}

/// Evaluate a - b, setting a flag if there was no underflow
fn subtract_with_underflow(a: u8, b:u8) -> (u8, bool){
    if b > a {
        (u8::MAX - b + a, false)
    } else {
        (a - b, true)
    }
}



pub(crate) const INSTRUCTION_SIZE: usize = 2;

/// Update the state of the emulator according to `instruction`
pub fn do_instruction(memory: &mut Memory, registers: &mut Registers){
    let instruction = get_instruction(memory, registers);
    match instruction {
        Instruction::Nop => (),
        Instruction::Jump(addr) => registers.pc = addr as usize,
        Instruction::Call(addr) => {            
            memory.ram[registers.sp-1..=registers.sp].copy_from_slice(&(registers.pc as u16).to_be_bytes());
            registers.sp -= 2;
            registers.pc = addr as usize
        }, 
        Instruction::Ret => {
            registers.sp += 2;
            let mut bytes :[u8;2] = [0,0];
            bytes.copy_from_slice(&memory.ram[registers.sp-1..=registers.sp]);
            registers.pc = u16::from_be_bytes(bytes) as usize;
        }
        Instruction::SkipEqImm(reg,imm ) => {
            if registers.vn[reg as usize] == imm {
                registers.pc += INSTRUCTION_SIZE;
            }
        }
        Instruction::SkipNeImm(reg,imm ) => {
            if registers.vn[reg as usize] != imm {
                registers.pc += INSTRUCTION_SIZE
            }
        }
        Instruction::SkipEqReg(r1,r2 ) => {
            if registers.vn[r1 as usize] == registers.vn[r2 as usize] {
                registers.pc += INSTRUCTION_SIZE
            }
        }
        Instruction::SkipNeReg(r1, r2 ) => {
            if registers.vn[r1 as usize] != registers.vn[r2 as usize] {
                registers.pc += INSTRUCTION_SIZE
            }
        }
        Instruction::SetImm(reg,imm) => registers.vn[reg as usize] = imm,
        Instruction::AddImm(reg,imm ) => {
            let (result, _ ) = add_with_overflow(registers.vn[reg as usize], imm);
            registers.vn[reg as usize] = result
        }
        Instruction::AddReg(vx, vy) => {
            let x: u8 = registers.vn[vx as usize];
            let y: u8 = registers.vn[vy as usize];
            let (result, flag) = add_with_overflow(x,y);
            registers.vn[15] = flag as u8;
            registers.vn[vx as usize] = result;
        }
        Instruction::SubReg(vx, vy) => {
            let (result, flag) = subtract_with_underflow(
                registers.vn[vx as usize], registers.vn[vy as usize]);
            registers.vn[vx as usize] = result;
            registers.vn[15 as usize] = flag as u8;
        }
        Instruction::SubFrom(vx, vy) => {
            let (result, flag) = subtract_with_underflow(vy, vx);
            registers.vn[vx as usize] = result;
            registers.vn[15 as usize] = flag as u8;
        }
        Instruction::ClearScreen => memory.display = [false; 64 * 32],
        Instruction::Draw(vx,vy ,n ) => {
            let x = registers.vn[vx as usize] as usize;
            let y = registers.vn[vy as usize] as usize;
            assert!(x < DISPLAY_ROWS);
            assert!(y < DISPLAY_COLUMNS);
            for count in 0..n as usize{
                let addr = registers.i + count;
                let sprite_row = memory.ram[addr];
                memory.set_row(x+count, y, sprite_row);
            }
        },
        Instruction::SetChar(reg) => {
            let char_index = registers.vn[reg as usize];
            assert!(char_index < 16);
            registers.i = char_index as usize * 5;
        }
        Instruction::SetMemPtr(imm) => {
            registers.i = imm as usize;
        }
        Instruction::WaitForKey(reg) => registers.key_flag = Some(reg as usize),
        Instruction::SetReg(r1, r2) => registers.vn[r1 as usize] = registers.vn[r2 as usize],
        Instruction::OrReg(r1, r2) => registers.vn[r1 as usize] |= registers.vn[r2 as usize],
        Instruction::AndReg(r1,r2) => registers.vn[r1 as usize] &= registers.vn[r2 as usize],
        Instruction::XorReg(r1, r2) => registers.vn[r1 as usize] ^= registers.vn[r2 as usize],
        Instruction::Rsh(r1) => {
            registers.vn[15] = registers.vn[r1 as usize] & 0x0001;
            registers.vn[r1 as usize] >>= 1;
        },
        Instruction::Lsh(r1) =>{
            registers.vn[15] =registers.vn[r1 as usize] & (1<<7);
             registers.vn[r1 as usize] <<= 1;
        },
        Instruction::JumpOffset(imm) => registers.pc = (registers.vn[0] as u16 + imm) as usize,
        Instruction::Rand(reg, imm) => registers.vn[reg as usize] = rand::random::<u8>() & imm,
        Instruction::SkipKey(reg) => if memory.keys[registers.vn[reg as usize] as usize ] {
            registers.pc += INSTRUCTION_SIZE
        },
        Instruction::SkipNoKey(reg) => if !memory.keys[registers.vn[reg as usize] as usize ] {
            registers.pc += INSTRUCTION_SIZE
        },
        Instruction::GetDelay(reg) => registers.vn[reg as usize] = registers.delay,
        Instruction::SetDelay(reg) => registers.delay = registers.vn[reg as usize],
        Instruction::SetSound(reg) => registers.sound = registers.vn[reg as usize],
        Instruction::AddMemPtr(reg) => registers.i += registers.vn[reg as usize] as usize, 
        Instruction::BCD(reg) => {
            let val = registers.vn[reg as usize];
            let ones = val % 10;
            let tens = val % 100 - ones;
            let hundreds = val - tens - ones;
            memory.ram[registers.i] = hundreds;
            memory.ram[registers.i+1] = tens;
            memory.ram[registers.i+2] = ones; 
        }
        Instruction::RegDump(reg) => {
                memory.ram[registers.i..registers.i + reg as usize].copy_from_slice(&registers.vn[0..reg as usize])
        }
        Instruction::RegLoad(vx) => {
            registers.vn[0..vx as usize].copy_from_slice(&memory.ram[registers.i..registers.i + vx as usize])
        }
    }
    if !matches!(instruction, Instruction::Jump(_) | Instruction::JumpOffset(_) | Instruction::Call(_) | Instruction::Ret){
        registers.pc += INSTRUCTION_SIZE;
    }
}


#[test]
fn test_instructions(){
    assert_eq!(<u16 as Into<Instruction>>::into(0x00E0_u16), Instruction::ClearScreen);
    assert_eq!(<u16 as Into<Instruction>>::into(0x1e35_u16), Instruction::Jump(0xe35));
    assert_eq!(<u16 as Into<Instruction>>::into(0x5e30_u16), Instruction::SkipEqReg(0xe, 0x3));
}


#[test]
fn test_instruction_in_memory() {
    let instructions: [u8; 6] = [0x00,0xe0, 0x1e, 0x35, 0x5e, 0x30];
    let mut memory = Memory::default();
    memory.ram[1024..1024 + instructions.len()].copy_from_slice(&instructions);

    let mut registers = Registers::default();
    registers.pc=1024;
    assert_eq!(get_instruction(&memory, &registers), Instruction::ClearScreen);
    registers.pc += INSTRUCTION_SIZE ;
    assert_eq!(get_instruction(&memory, &registers), Instruction::Jump(0xe35));
    registers.pc += INSTRUCTION_SIZE;
    assert_eq!(get_instruction(&memory, &registers), Instruction::SkipEqReg(0xe, 0x3))
}


#[test]
fn test_jump() {
    let rom = [0x12, 0x04, 0x00, 0x00, 0x12, 0x00];
    let mut memory = Memory::default();
    memory.load_rom(&rom);
    let mut registers = Registers::default();
    do_instruction(&mut memory, &mut registers);
    assert_eq!(registers.pc, 0x204);
    do_instruction(&mut memory, &mut registers);
    assert_eq!(registers.pc, 0x200)
}


#[test]
fn test_call_ret() {
    let rom = [0x22, 0x04, 0x00, 0x00, 0x00, 0xEE];
    let mut memory = Memory::default();
    memory.load_rom(&rom);
    let mut registers = Registers::default();
    assert_eq!(registers.sp, 0x1ff);
    do_instruction(&mut memory, &mut registers);
    assert_eq!(registers.pc, 0x204);
    assert_eq!(registers.sp, 0x1fd);
    do_instruction(&mut memory, &mut registers);
    assert_eq!(registers.sp, 0x1ff);
    assert_eq!(registers.pc, 0x200)
}


