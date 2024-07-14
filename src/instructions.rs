
use crate::Instruction;
use crate::errors::ParseError;

macro_rules! get_arg {
    ($parts: expr, $index: expr) => {
        {
        let __arg_part = $parts.get($index);
        match __arg_part {
            Some(__arg_str) => {
                if let Ok(__val) = __arg_str.parse() {
                    Ok(__val)
                } else {Err(crate::errors::ParseError::new(&$parts.join(" "), &format!("Couldn't parse value {}", __arg_str)))}
            },
            None => Err(crate::errors::ParseError::new(&$parts.join(" "), &format!("Missing argument {}", $index)))
        }}
    }
}

impl Instruction{
    pub fn from_mnemonic(mnemonic: &str) -> Result<Instruction,ParseError> {
        let lower = mnemonic.to_ascii_lowercase();
        let mnemonic_parts:Vec<_> = lower.split(|s:char|{s.is_whitespace()}).collect();
        Ok(
            match mnemonic_parts[0] {
            "cls" => Instruction::ClearScreen,
            "ret" => Instruction::Ret,
            "nop" => Instruction::Nop,
            "jmp" => Instruction::Jump(get_arg!(mnemonic_parts, 1)?),
            "call" => Instruction::Call(get_arg!(mnemonic_parts, 1)?),
            "skei" => Instruction::SkipEqImm(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "skni" => Instruction::SkipNeImm(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "skev" => Instruction::SkipEqReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "movi" => Instruction::SetImm(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "addi" => Instruction::AddImm(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "movv" => Instruction::SetReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "or" => Instruction::OrReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "and" => Instruction::AndReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "xor" => Instruction::XorReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "addv" => Instruction::AddReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "subv" => Instruction::SubReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "rsh" => Instruction::Rsh(get_arg!(mnemonic_parts, 1)?),
            "subf" => Instruction::SubFrom(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "lsh" => Instruction::Lsh(get_arg!(mnemonic_parts, 1)?),
            "sknv" => Instruction::SkipNeReg(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "movm" => Instruction::SetMemPtr(get_arg!(mnemonic_parts, 1)?),
            "joff" => Instruction::JumpOffset(get_arg!(mnemonic_parts, 1)?),
            "rnd" | "rand" => Instruction::Rand(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?),
            "draw" => Instruction::Draw(get_arg!(mnemonic_parts, 1)?, get_arg!(mnemonic_parts, 2)?, get_arg!(mnemonic_parts, 3)?),
            "skk" => Instruction::SkipKey(get_arg!(mnemonic_parts, 1)?),
            "snk" => Instruction::SkipNoKey(get_arg!(mnemonic_parts, 1)?),
            "getd" => Instruction::GetDelay(get_arg!(mnemonic_parts, 1)?),
            "wait" => Instruction::WaitForKey(get_arg!(mnemonic_parts, 1)?),
            "movd" => Instruction::SetDelay(get_arg!(mnemonic_parts, 1)?),
            "movs" => Instruction::SetSound(get_arg!(mnemonic_parts, 1)?),
            "addm" => Instruction::AddMemPtr(get_arg!(mnemonic_parts, 1)?),
            "movc" => Instruction::SetChar(get_arg!(mnemonic_parts, 1)?),
            "bcd" => Instruction::BCD(get_arg!(mnemonic_parts, 1)?),
            "rdump" | "rdmp" => Instruction::RegDump(get_arg!(mnemonic_parts, 1)?),
            "rload" => Instruction::RegLoad(get_arg!(mnemonic_parts, 1)?),
            _=> { return Err(ParseError::new(mnemonic, "Unknown instruction"))}
        })
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

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Nop => write!(f, "NOP"),
            Instruction::ClearScreen => write!(f, "CLS"),
            Instruction::Ret => write!(f, "RET"),
            Instruction::Jump(v) => write!(f, "JMP {}", v),
            Instruction::Call(v) => write!(f, "CALL {}", v),
            Instruction::SkipEqImm(reg, imm) => write!(f, "SKEI {} {}", reg, imm),
            Instruction::SkipNeImm(reg, imm ) =>  write!(f, "SKNI {} {}", reg, imm),
            Instruction::SkipEqReg(r1, r2 ) => write!(f, "SKEV {} {}", r1, r2),
            Instruction::SetImm(reg, imm) => write!(f, "MOVI {reg} {imm}"),
            Instruction::AddImm(reg, imm) => write!(f, "ADDI {reg} {imm}"),
            Instruction::SetReg(r1,r2) => write!(f, "MOVV {r1} {r2}"),
            Instruction::OrReg(r1,r2 ) => write!(f, "OR {r1} {r2}"),
            Instruction::AndReg(r1,r2 ) => write!(f, "AND {r1} {r2}"),
            Instruction::XorReg(r1,r2 ) => write!(f, "XOR {r1} {r2}"),
            Instruction::AddReg(r1,r2 ) => write!(f, "ADDV {r1} {r2}"),
            Instruction::SubReg(r1,r2 ) => write!(f, "SUBV {r1} {r2}"),
            Instruction::Rsh(r1 ) => write!(f, "RSH {r1}"),
            Instruction::SubFrom(r1,r2 ) => write!(f, "SUBF {r1} {r2}"),
            Instruction::Lsh(r1 ) => write!(f, "LSH {r1}"),
            Instruction::SkipNeReg(r1,r2 )=> write!(f, "SKNV {r1} {r2}"),
            Instruction::SetMemPtr(imm) => write!(f, "MOVM {imm}"),
            Instruction::JumpOffset(imm) => write!(f, "JOFF {imm}"),
            Instruction::Rand(reg,imm ) => write!(f, "RAND {reg} {imm}"),
            Instruction::Draw(x,y ,n) => write!(f, "DRAW {x} {y} {n}"),
            Instruction::SkipKey(reg) => write!(f, "SKK {reg}"),
            Instruction::SkipNoKey(reg) => write!(f, "SNK {reg}"),
            Instruction::GetDelay(reg) => write!(f, "GETD {reg}"),
            Instruction::WaitForKey(reg) => write!(f, "WAIT {reg}"),
            Instruction::SetDelay(reg) => write!(f, "MOVD {reg}"),
            Instruction::SetSound(reg) => write!(f, "MOVS {reg}"),
            Instruction::AddMemPtr(reg) => write!(f, "ADDM {reg}"),
            Instruction::SetChar(reg) => write!(f, "MOVC {reg}"),
            Instruction::BCD(reg) => write!(f, "BCD {reg}"),
            Instruction::RegDump(reg) => write!(f, "RDMP {reg}"),
            Instruction::RegLoad(reg) => write!(f, "RLOAD {reg}")
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

