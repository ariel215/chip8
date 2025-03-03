use crate::*;

/////////////////////////////////////
/// Memory
/////////////////////////////////////

#[inline]
fn get_bit(char: u8, index: usize) -> bool{
    assert!(index < 8);
    let mask = 1 << (7-index);
    let masked_char = char & mask;
    (masked_char >> (7-index)) == 1
}

#[test]
fn test_bytes(){
    let test:u8 = 0b01001011;
    let test_bools = [false, true, false ,false, true, false, true, true];
    for i in 0..8{
        assert_eq!(get_bit(test, i), test_bools[i], "test failed on case {i}");
    }
}

impl Default for Memory {
    fn default() -> Self {
        let mut mem = Self { 
            ram: [0;4096], 
            display: Display::from_elem([DISPLAY_COLUMNS, DISPLAY_ROWS], false),
            keys: Default::default(),
            stack: Default::default()
        };
        mem.ram[0..CHAR_SPRITES.len()].copy_from_slice(&CHAR_SPRITES);
        mem
    }
}

impl Memory{
    pub fn load_rom(&mut self, rom: &[u8]){
        self.ram[0x200..0x200 + rom.len()].copy_from_slice(rom);
    }
    
    /// Xor a row of 8 pixels starting at (x,y) with the pattern in `byte`,
    /// returning True if a pixel was flipped from on to off
    fn set_row(&mut self, x: usize, y: usize, byte: u8) -> bool{
        let mut collided = false;
        for i in 0..8 {
            let x_wrapped = (x+i) % DISPLAY_COLUMNS;
            let y_wrapped = y % DISPLAY_ROWS;
            let prev = self.display[[x_wrapped, y_wrapped]];
            let new = get_bit(byte, i);
            self.display[[x_wrapped,y_wrapped]] ^= new;
            collided |= prev & new;
        }
        collided
    }
}


#[test]
fn test_set_row(){
    let mut memory = Memory::default();
    let collision = memory.set_row(0, 0, 0xff);
    assert!(!collision);
    let collision = memory.set_row(0,0,0xff);
    assert!(collision);
    let collision = memory.set_row(0, 0, 0xff);
    assert!(!collision)
}


////////////////////////////
///  Registers
//////////////////////////////

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

//////////////////////////////
/// Emulator
/////////////////////////////


impl Chip8{
    pub const DEFAULT_SPEED: u64 = 500;


    pub fn init(speed: Option<u64>)->Self{
        Self{
            clock_speed: speed.unwrap_or(Self::DEFAULT_SPEED),
            memory: Memory::default(),
            registers: Registers::default(),
        }
    }

    pub fn clock_speed(&mut self, speed: u64){
        self.clock_speed = speed;
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.memory.load_rom(rom);
    }

    pub fn do_instruction(&mut self){
        if self.registers.key_flag.is_none() {
            do_instruction(&mut self.memory, &mut self.registers)
        }
        
    }

    pub fn tick_timers(&mut self){
        if self.registers.delay > 0{
            self.registers.delay -= 1;
        }
        if self.registers.sound >0 {
            self.registers.sound -= 1;
        }
    }

    pub fn set_key(&mut self, key: u8){
        self.memory.keys[key as usize] = true;
        if let Some(dest) = self.registers.key_flag{
            self.registers.vn[dest] = key;
            self.registers.key_flag = None;
        }
    }

    pub fn clear_keys(&mut self){
        self.memory.keys = [false; 16]
    }

    pub fn sound(&self) -> bool {
        self.registers.sound > 0
    }

    pub fn pc(&self) -> usize {
        self.registers.pc
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

pub const INSTRUCTION_SIZE: usize = 2;

/// Update the state of the emulator according to `instruction`
pub(crate) fn do_instruction(memory: &mut Memory, registers: &mut Registers){
    let instruction = get_instruction(memory, registers);
    match instruction {
        Instruction::Nop => (),
        Instruction::Jump(addr) => registers.pc = addr as usize,
        Instruction::Call(addr) => {  
            memory.stack.push(registers.pc);
            registers.pc = addr as usize;          
        }, 
        Instruction::Ret => {
            registers.pc = memory.stack.pop().expect("Returning from empty call stack")
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
            registers.vn[15] = flag as u8;
        }
        Instruction::SubFrom(vx, vy) => {
            let (result, flag) = subtract_with_underflow(vy, vx);
            registers.vn[vx as usize] = result;
            registers.vn[15] = flag as u8;
        }
        Instruction::ClearScreen => memory.display.fill(false),
        // Draws n bytes from memory on screen
        // Successive bytes are drawn one below the next
        Instruction::Draw(vx,vy ,n ) => {
            let x = registers.vn[vx as usize] as usize;
            let y = registers.vn[vy as usize] as usize;
            // assert!(x < DISPLAY_COLUMNS);
            // assert!(y < DISPLAY_ROWS);
            let mut collided = false;
            for count in 0..n as usize{
                let addr = registers.i + count;
                let sprite_row = memory.ram[addr];
                collided |= memory.set_row(x, y+count, sprite_row);
            }
            registers.vn[15] = collided as u8;
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
        Instruction::SkipKeyPressed(reg) => if memory.keys[registers.vn[reg as usize] as usize ] {
            registers.pc += INSTRUCTION_SIZE
        },
        Instruction::SkipKeyNotPressed(reg) => if !memory.keys[registers.vn[reg as usize] as usize ] {
            registers.pc += INSTRUCTION_SIZE
        },
        Instruction::GetDelay(reg) => registers.vn[reg as usize] = registers.delay,
        Instruction::SetDelay(reg) => registers.delay = registers.vn[reg as usize],
        Instruction::SetSound(reg) => registers.sound = registers.vn[reg as usize],
        Instruction::AddMemPtr(reg) => registers.i += registers.vn[reg as usize] as usize, 
        Instruction::BCD(reg) => {
            let val = registers.vn[reg as usize];
            let ones = val % 10;
            let tens = (val % 100) / 10;
            let hundreds = (val - tens - ones) / 100;
            memory.ram[registers.i] = hundreds;
            memory.ram[registers.i+1] = tens;
            memory.ram[registers.i+2] = ones; 
        }
        Instruction::RegDump(reg) => {
                memory.ram[registers.i..(registers.i + reg as usize) + 1].copy_from_slice(&registers.vn[0..(reg+1) as usize])
        }
        Instruction::RegLoad(vx) => {
            registers.vn[0..vx as usize + 1].copy_from_slice(&memory.ram[registers.i..registers.i + vx as usize + 1])
        }
    }
    if !matches!(instruction, Instruction::Jump(_) | Instruction::JumpOffset(_) | Instruction::Call(_) ){
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
    assert_eq!(registers.pc, 0x200);
    do_instruction(&mut memory, &mut registers);
    assert_eq!(registers.pc, 0x204);
    assert_eq!(memory.stack.len(), 1);
    assert_eq!(memory.stack[memory.stack.len()-1],0x200);
    do_instruction(&mut memory, &mut registers);
    assert_eq!(registers.pc, 0x202);
    assert_eq!(memory.stack.len(),0)
}

#[test]
fn test_load_char(){
    let instrs: Vec<u16> = ["ld v0 1", "ld v1 0", "ld v2 0", "ld f 0", "drw 1 2 5"].into_iter().map(
        |s| Instruction::from_mnemonic(s).unwrap().into()
    ).collect();
    let rom: Vec<u8> = instrs.iter().flat_map(|s|s.to_be_bytes()).collect();
    let mut memory = Memory::default();
    memory.load_rom(&rom);
    let mut registers = Registers::default();
    for _ in 0..instrs.len() {
        do_instruction(&mut memory, &mut registers);
    }
    assert!(memory.display[[2,0]]);                                 // xx*x
    assert!(memory.display[[1,1]]); assert!(memory.display[[2,1]]); // x**x
    assert!(memory.display[[2,2]]);                                 // xx*x
    assert!(memory.display[[2,3]]);                                 // xx*x
    let slice = s![1..4,4];    // x***
    assert!(memory.display.slice(slice).iter().all(|f|*f))
}
