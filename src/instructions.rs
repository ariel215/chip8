use crate::errors::ParseError;
type Addr = u16;
type Reg = u8;

#[derive(Debug, PartialEq, Eq)]
// Todo: turn these docs into attributes for a proc macro
// ideally could derive: instruction->mnemonic, mnemonic-> instruction,
// instruction -> u16, u16 -> instruction
// all from attributes
pub enum Instruction {
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
    SkipEqImm(Reg, u8),
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
    Draw(Reg, Reg, u8),
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


macro_rules! get_arg {
    ($parts: expr, $index: expr) => {{
        let __arg_part = $parts.get($index);
        match __arg_part {
            Some(__arg_str) => {
                let __arg_str = if let Some(__suffix) = __arg_str.strip_prefix("v") {
                    __suffix
                } else {
                    __arg_str
                };
                if let Ok(__val) = __arg_str.parse() {
                    Ok(__val)
                } else {
                    Err(crate::errors::ParseError::new(
                        &$parts.join(" "),
                        &format!("Couldn't parse value {}", __arg_str),
                    ))
                }
            }
            None => Err(crate::errors::ParseError::new(
                &$parts.join(" "),
                &format!("Missing argument {}", $index),
            )),
        }
    }};
}

impl Instruction {
    pub fn from_mnemonic(mnemonic: &str) -> Result<Instruction, ParseError> {
        let lower = mnemonic.to_ascii_lowercase();
        let mnemonic_parts: Vec<_> = lower.split(|s: char| s.is_whitespace()).collect();
        Ok(match mnemonic_parts[0] {
            "cls" => Instruction::ClearScreen,
            "ret" => Instruction::Ret,
            "nop" => Instruction::Nop,
            "jp" => {
                if mnemonic_parts.len() == 1 {
                    Instruction::Jump(get_arg!(mnemonic_parts, 1)?)
                } else {
                    Instruction::JumpOffset(get_arg!(mnemonic_parts, 2)?)
                }
            }
            "call" => Instruction::Call(get_arg!(mnemonic_parts, 1)?),
            "se" => {
                if mnemonic_parts[2].starts_with('V') {
                    Instruction::SkipEqReg(
                        get_arg!(mnemonic_parts, 1)?,
                        get_arg!(mnemonic_parts, 2)?,
                    )
                } else {
                    Instruction::SkipEqImm(
                        get_arg!(mnemonic_parts, 1)?,
                        get_arg!(mnemonic_parts, 2)?,
                    )
                }
            }
            "sne" => {
                if mnemonic_parts[2].starts_with('V') {
                    Instruction::SkipNeReg(
                        get_arg!(mnemonic_parts, 1)?,
                        get_arg!(mnemonic_parts, 2)?,
                    )
                } else {
                    Instruction::SkipNeImm(
                        get_arg!(mnemonic_parts, 1)?,
                        get_arg!(mnemonic_parts, 2)?,
                    )
                }
            }
            "ld" => match mnemonic_parts[1] {
                "i" => Instruction::SetMemPtr(get_arg!(mnemonic_parts, 2)?),
                "[i]" => Instruction::RegDump(get_arg!(mnemonic_parts, 2)?),
                "dt" => Instruction::SetDelay(get_arg!(mnemonic_parts, 2)?),
                "st" => Instruction::SetSound(get_arg!(mnemonic_parts, 2)?),
                "f" => Instruction::SetChar(get_arg!(mnemonic_parts, 2)?),
                "b" => Instruction::BCD(get_arg!(mnemonic_parts, 2)?),
                _ => match mnemonic_parts[2] {
                    "dt" => Instruction::GetDelay(get_arg!(mnemonic_parts, 1)?),
                    "k" => Instruction::WaitForKey(get_arg!(mnemonic_parts, 1)?),
                    "[i]" => Instruction::RegLoad(get_arg!(mnemonic_parts, 1)?),
                    _ => {
                        if mnemonic_parts[2].starts_with('v') {
                            Instruction::SetReg(
                                get_arg!(mnemonic_parts, 1)?,
                                get_arg!(mnemonic_parts, 2)?,
                            )
                        } else {
                            Instruction::SetImm(
                                get_arg!(mnemonic_parts, 1)?,
                                get_arg!(mnemonic_parts, 2)?,
                            )
                        }
                    }
                },
            },
            "add" => {
                if mnemonic_parts[1] == "I" {
                    Instruction::AddMemPtr(get_arg!(mnemonic_parts, 1)?)
                } else if mnemonic_parts[2].starts_with('V') {
                    Instruction::AddReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?)
                } else {
                    Instruction::AddImm(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?)
                }
            }
            "or" => Instruction::OrReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "and" => {
                Instruction::AndReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?)
            }
            "xor" => {
                Instruction::XorReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?)
            }
            "sub" => {
                Instruction::SubReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?)
            }
            "rsh" => Instruction::Rsh(get_arg!(mnemonic_parts, 1)?),
            "subn" => {
                Instruction::SubFrom(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?)
            }
            "lsh" => Instruction::Lsh(get_arg!(mnemonic_parts, 1)?),
            "rnd" => Instruction::Rand(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "drw" => Instruction::Draw(
                get_arg!(mnemonic_parts, 1)?,
                get_arg!(mnemonic_parts, 2)?,
                get_arg!(mnemonic_parts, 3)?,
            ),
            "skp" => Instruction::SkipKeyPressed(get_arg!(mnemonic_parts, 1)?),
            "sknp" => Instruction::SkipKeyNotPressed(get_arg!(mnemonic_parts, 1)?),
            _ => return Err(ParseError::new(mnemonic, "Unknown instruction")),
        })
    }
}

// First register argument
macro_rules! X {
    ($opcode: expr) => {{
        let reg = ($opcode & 0x0F00) >> 8;
        assert!(reg < u8::MAX.into());
        reg as u8
    }};
}

// Second register argument
macro_rules! Y {
    ($opcode: expr) => {{
        let reg = ($opcode & 0x00F0) >> 4;
        assert!(reg < u8::MAX.into());
        reg as u8
    }};
}

// 4-bit immediate
macro_rules! N {
    ($opcode: expr) => {
        ($opcode & 0x000F) as u8
    };
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
        ($r1 as u16) << 8 | (($r2 as u16) << 4)
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
            Instruction::SkipNeImm(reg, imm) => 0x4000 | XNN!(reg, imm),
            Instruction::SkipEqReg(r1, r2) => 0x5000 | XY!(r1, r2),
            Instruction::SetImm(reg, imm) => 0x6000 | XNN!(reg, imm),
            Instruction::AddImm(reg, imm) => 0x7000 | XNN!(reg, imm),
            Instruction::SetReg(r1, r2) => 0x8000 | XY!(r1, r2) | 0,
            Instruction::OrReg(r1, r2) => 0x8000 | XY!(r1, r2) | 1,
            Instruction::AndReg(r1, r2) => 0x8000 | XY!(r1, r2) | 2,
            Instruction::XorReg(r1, r2) => 0x8000 | XY!(r1, r2) | 3,
            Instruction::AddReg(r1, r2) => 0x8000 | XY!(r1, r2) | 4,
            Instruction::SubReg(r1, r2) => 0x8000 | XY!(r1, r2) | 5,
            Instruction::Rsh(r1) => 0x8000 | XY!(r1, 0) | 6,
            Instruction::SubFrom(r1, r2) => 0x8000 | XY!(r1, r2) | 7,
            Instruction::Lsh(r1) => 0x8000 | XY!(r1, 0) | 0xe,
            Instruction::SkipNeReg(r1, r2) => 0x9000 | XY!(r1, r2),
            Instruction::SetMemPtr(imm) => 0xA000 | imm,
            Instruction::JumpOffset(imm) => 0xB000 | imm,
            Instruction::Rand(reg, imm) => 0xC000 | XNN!(reg, imm),
            Instruction::Draw(x, y, n) => 0xD000 | XY!(x, y) | n as u16,
            Instruction::SkipKeyPressed(reg) => 0xE09E | XY!(reg, 0),
            Instruction::SkipKeyNotPressed(reg) => 0xE0A1 | XY!(reg, 0),
            Instruction::GetDelay(reg) => 0xF007 | XY!(reg, 0),
            Instruction::WaitForKey(reg) => 0xF00A | XY!(reg, 0),
            Instruction::SetDelay(reg) => 0xF015 | XY!(reg, 0),
            Instruction::SetSound(reg) => 0xF018 | XY!(reg, 0),
            Instruction::AddMemPtr(reg) => 0xF01E | XY!(reg, 0),
            Instruction::SetChar(reg) => 0xF029 | XY!(reg, 0),
            Instruction::BCD(reg) => 0xF033 | XY!(reg, 0),
            Instruction::RegDump(reg) => 0xF055 | XY!(reg, 0),
            Instruction::RegLoad(reg) => 0xF065 | XY!(reg, 0),
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Nop => write!(f, "NOP"),
            Instruction::ClearScreen => write!(f, "CLS"),
            Instruction::Ret => write!(f, "RET"),
            Instruction::Jump(v) => write!(f, "JMP V{}", v),
            Instruction::Call(v) => write!(f, "CALL V{}", v),
            Instruction::SkipEqImm(reg, imm) => write!(f, "SE V{} {}", reg, imm),
            Instruction::SkipEqReg(r1, r2) => write!(f, "SE V{} V{}", r1, r2),
            Instruction::SkipNeImm(reg, imm) => write!(f, "SNE V{} {}", reg, imm),
            Instruction::SkipNeReg(r1, r2) => write!(f, "SNE V{r1} V{r2}"),
            Instruction::SetImm(reg, imm) => write!(f, "LD V{reg} {imm}"),
            Instruction::AddImm(reg, imm) => write!(f, "ADD V{reg} {imm}"),
            Instruction::AddMemPtr(reg) => write!(f, "ADD I V{reg}"),
            Instruction::AddReg(r1, r2) => write!(f, "ADD V{r1} V{r2}"),
            Instruction::SetReg(r1, r2) => write!(f, "LD V{r1} V{r2}"),
            Instruction::OrReg(r1, r2) => write!(f, "OR V{r1} V{r2}"),
            Instruction::AndReg(r1, r2) => write!(f, "AND V{r1} V{r2}"),
            Instruction::XorReg(r1, r2) => write!(f, "XOR V{r1} V{r2}"),
            Instruction::SubReg(r1, r2) => write!(f, "SUB V{r1} V{r2}"),
            Instruction::Rsh(r1) => write!(f, "RSH V{r1}"),
            Instruction::SubFrom(r1, r2) => write!(f, "SUBN V{r1} V{r2}"),
            Instruction::Lsh(r1) => write!(f, "LSH V{r1}"),
            Instruction::SetMemPtr(imm) => write!(f, "LD I {imm}"),
            Instruction::JumpOffset(imm) => write!(f, "JP V0 {imm}"),
            Instruction::Rand(reg, imm) => write!(f, "RND V{reg} {imm}"),
            Instruction::Draw(x, y, n) => write!(f, "DRW V{x} V{y} {n}"),
            Instruction::SkipKeyPressed(reg) => write!(f, "SKP V{reg}"),
            Instruction::SkipKeyNotPressed(reg) => write!(f, "SKNP V{reg}"),
            Instruction::GetDelay(reg) => write!(f, "LD V{reg} DT"),
            Instruction::WaitForKey(reg) => write!(f, "LD V{reg} K"),
            Instruction::SetDelay(reg) => write!(f, "LD DT V{reg}"),
            Instruction::SetSound(reg) => write!(f, "LD ST V{reg}"),
            Instruction::SetChar(reg) => write!(f, "LD F V{reg}"),
            Instruction::BCD(reg) => write!(f, "LD B V{reg}"),
            Instruction::RegDump(reg) => write!(f, "LD [I] V{reg}"),
            Instruction::RegLoad(reg) => write!(f, "LD V{reg} [I]"),
        }
    }
}

impl From<u16> for Instruction {
    fn from(opcode: u16) -> Self {
        match opcode {
            0x00E0 => Self::ClearScreen,
            0x00EE => Self::Ret,
            0x1000..=0x1fff => Self::Jump(NNN!(opcode)),
            0x2000..=0x2fff => Self::Call(NNN!(opcode)),
            0x3000..=0x3fff => Self::SkipEqImm(X!(opcode), NN!(opcode)),
            0x4000..=0x4fff => Self::SkipNeImm(X!(opcode), NN!(opcode)),
            0x5000..=0x5ff0 => Self::SkipEqReg(X!(opcode), Y!(opcode)),
            0x6000..=0x6fff => Self::SetImm(X!(opcode), NN!(opcode)),
            0x7000..=0x7fff => Self::AddImm(X!(opcode), NN!(opcode)),
            0x8000..=0x8fff => match opcode & 0x000f {
                0x0000 => Self::SetReg(X!(opcode), Y!(opcode)),
                0x0001 => Self::OrReg(X!(opcode), Y!(opcode)),
                0x0002 => Self::AndReg(X!(opcode), Y!(opcode)),
                0x0003 => Self::XorReg(X!(opcode), Y!(opcode)),
                0x0004 => Self::AddReg(X!(opcode), Y!(opcode)),
                0x0005 => Self::SubReg(X!(opcode), Y!(opcode)),
                0x0006 => Self::Rsh(X!(opcode)),
                0x0007 => Self::SubFrom(X!(opcode), Y!(opcode)),
                0x000E => Self::Lsh(X!(opcode)),
                _ => Self::Nop,
            },
            0x9000..=0x9ff0 => Self::SkipNeReg(X!(opcode), Y!(opcode)),
            0xA000..=0xAfff => Self::SetMemPtr(NNN!(opcode)),
            0xB000..=0xBfff => Self::JumpOffset(NNN!(opcode)),
            0xC000..=0xCfff => Self::Rand(X!(opcode), NN!(opcode)),
            0xD000..=0xDfff => Self::Draw(X!(opcode), Y!(opcode), N!(opcode)),
            0xE000..=0xEfff => {
                let lower = NN!(opcode);
                match lower {
                    0x9E => Self::SkipKeyPressed(X!(opcode)),
                    0xA1 => Self::SkipKeyNotPressed(X!(opcode)),
                    _ => Self::Nop,
                }
            }
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
                    _ => Self::Nop,
                }
            }
            _ => Self::Nop,
        }
    }
}
