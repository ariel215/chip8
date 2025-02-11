use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    ops::Range,
};

use chip8::{Instruction, INSTRUCTION_SIZE};
use itertools::Itertools;
use pest::{
    error::ErrorVariant,
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/labels.pest"]
pub struct InstructionParser;
pub type Error = pest::error::Error<Rule>;

#[derive(Debug, PartialEq)]
pub(crate) enum Line {
    Instr(Instruction),
    Data(Vec<u8>),
}

pub type ParseResult<'a> = Result<(Line, Option<&'a str>), Error>;

pub struct Program<'a> {
    /// The program instructions in the order that they appear.
    instructions: Vec<Line>,
    /// Maps textual labels to the addresses they point to
    labels: HashMap<&'a str, usize>,
    /// A map from each instruction to the label it uses, if any
    /// Invariant:
    /// `references[i]` is `Some(label)` iff `instructions[i]` is
    /// `call (label)` or `jmp (label)`
    references: Vec<Option<&'a str>>,
}

impl<'a> Program<'a> {
    /// replace every use of a label with that label's address
    pub fn fix_references(&mut self) {
        for (usage, label) in self.references.iter().enumerate() {
            if let Some(label) = *label {
                let target = self.labels[label];
                let sizes = &self.instructions[..target]
                    .iter()
                    .map(|l| match l {
                        Line::Instr(_) => INSTRUCTION_SIZE,
                        Line::Data(data) => data.len(),
                    })
                    .collect_vec();
                let total_size = sizes.into_iter().sum::<usize>();
                let new_addr = (total_size + 0x200) as u16;
                let new_instruction = match self.instructions[usage] {
                    Line::Instr(Instruction::Call(_)) => Instruction::Call(new_addr),
                    Line::Instr(Instruction::Jump(_)) => Instruction::Jump(new_addr),
                    _ => panic!(
                        "unexpected instruction: {:?} at {:?}",
                        self.instructions[usage], usage
                    ),
                };
                self.instructions[usage] = Line::Instr(new_instruction);
            }
        }
    }

    pub fn compile(&self) -> Vec<u8> {
        let mut bytes = vec![];
        for line in self.instructions.iter() {
            match line {
                Line::Instr(instruction) => {
                    bytes.extend(<Instruction as Into<u16>>::into(*instruction).to_be_bytes());
                }
                Line::Data(data) => {
                    bytes.extend_from_slice(data);
                }
            }
        }
        bytes
    }
}

fn error<'a, T>(rule: &Pair<'a, Rule>, message: &str) -> Result<T, Error> {
    return Err(Error::new_from_pos(
        ErrorVariant::CustomError {
            message: format!("{}: {:?}", message, rule),
        },
        rule.as_span().start_pos(),
    ));
}

/// Rule for parsing registers
fn register<'a>(name: &Pair<'a, Rule>) -> Result<u8, Error> {
    let s = name.as_str();
    if let Some(reg) = s.to_ascii_lowercase().strip_prefix('v') {
        reg.parse().or(Err(Error::new_from_span(
            ErrorVariant::CustomError {
                message: "invalid register name".to_string(),
            },
            name.as_span(),
        )))
    } else {
        Err(Error::new_from_span(
            ErrorVariant::CustomError {
                message: "invalid register name".to_string(),
            },
            name.as_span(),
        ))
    }
}

fn parse_u16<'a>(number: &Pair<'a, Rule>) -> Result<u16, Error> {
    match number.clone().into_inner().next().unwrap().as_rule() {
        Rule::decimal => Ok(number.as_str().parse().unwrap()),
        Rule::hex => {
            if let Ok(num) = u16::from_str_radix(number.as_str().strip_prefix("0x").unwrap(), 16) {
                Ok(num)
            } else {
                error(number, "expected u16")
            }
        }
        _ => error(number, "Unexpected rule"),
    }
}

fn parse_u8<'a>(number: &Pair<'a, Rule>) -> Result<u8, Error> {
    let bignum = parse_u16(number)?;
    if bignum > u8::MAX.into() {
        error(number, "Argument must be a single byte")
    } else {
        Ok(bignum as u8)
    }
}

/// Rule for parsing binary operations
fn parse_binop<'a>(pair: Pair<'a, Rule>) -> ParseResult<'a> {
    let rule = pair.as_rule();
    let mut parts = pair.into_inner();
    if matches!(rule, Rule::add) {
        if let Some(ref reg) = parts.find_first_tagged("addi") {
            return Ok((Line::Instr(Instruction::AddMemPtr(register(reg)?)), None));
        }
    }
    let r1 = register(&parts.next().unwrap())?;
    let p2 = &parts.next().unwrap();
    let r2 = match p2.as_rule() {
        Rule::register => register(p2)?,
        Rule::number => parse_u8(p2)?,
        _ => error(p2, "unexpected rule")?,
    };
    Ok((
        Line::Instr(match rule {
            Rule::add => {
                if matches!(p2.as_rule(), Rule::number) {
                    Instruction::AddImm(r1, r2)
                } else {
                    Instruction::AddReg(r1, r2)
                }
            }
            Rule::se => {
                if matches!(p2.as_rule(), Rule::number) {
                    Instruction::SkipEqImm(r1, r2)
                } else {
                    Instruction::SkipEqReg(r1, r2)
                }
            }
            Rule::sne => {
                if matches!(p2.as_rule(), Rule::number) {
                    Instruction::SkipNeImm(r1, r2)
                } else {
                    Instruction::SkipNeReg(r1, r2)
                }
            }
            Rule::sub => Instruction::SubReg(r1, r2),
            Rule::or => Instruction::OrReg(r1, r2),
            Rule::and => Instruction::AndReg(r1, r2),
            Rule::xor => Instruction::XorReg(r1, r2),
            Rule::subn => Instruction::SubFrom(r1, r2),
            _ => error(p2, "unexpected rule")?,
        }),
        None,
    ))
}

fn parse_addr<'a>(addr: Pairs<'a, Rule>) -> (u16, Option<&'a str>) {
    if let Some(fixed_addr) = addr.find_first_tagged("fixed") {
        let fixed_addr = fixed_addr.as_str().parse().unwrap();
        return (fixed_addr, None);
    }
    if let Some(label) = addr.find_first_tagged("label") {
        return (0, Some(label.as_str()));
    }
    unreachable!()
}

fn parse_call<'a>(mut call: Pairs<'a, Rule>) -> ParseResult {
    let addr = call.nth(1).unwrap().into_inner();
    let addr = parse_addr(addr);
    return Ok((Line::Instr(Instruction::Call(addr.0)), addr.1));
}

fn parse_jump<'a>(jump: Pair<'a, Rule>) -> ParseResult {
    let rule = jump.as_rule();
    let mut jump_in = jump.clone().into_inner();
    let addr = jump_in.next().unwrap().into_inner();
    let addr = parse_addr(addr);
    Ok((
        Line::Instr(match rule {
            Rule::jump => Instruction::Jump(addr.0),
            Rule::jpoff => Instruction::JumpOffset(addr.0),
            _ => error(&jump, "unexpected rule")?,
        }),
        addr.1,
    ))
}

fn parse_draw<'a>(mut draw: Pairs<'a, Rule>) -> ParseResult {
    let r1 = register(&draw.nth(1).unwrap())?;
    let r2 = register(&draw.nth(1).unwrap())?;
    let n = draw.nth(1).unwrap().as_str().parse().unwrap();
    return Ok((Line::Instr(Instruction::Draw(r1, r2, n)), None));
}

fn parse_unop<'a>(op: Pair<'a, Rule>) -> ParseResult {
    let rule = op.as_rule();
    let reg = register(&op.clone().into_inner().nth(1).unwrap())?;
    Ok((
        Line::Instr(match rule {
            Rule::rsh => Instruction::Rsh(reg),
            Rule::lsh => Instruction::Lsh(reg),
            Rule::skp => Instruction::SkipKeyPressed(reg),
            Rule::sknp => Instruction::SkipKeyNotPressed(reg),
            _ => error(&op, "unexpected rule")?,
        }),
        None,
    ))
}

fn parse_load<'a>(load_args: Pair<'a, Rule>) -> ParseResult<'a> {
    let pair = load_args.into_inner().next().unwrap();
    let arg0 = pair.clone().into_inner().next().unwrap();
    Ok((
        Line::Instr(match pair.as_rule() {
            Rule::ldchar => Instruction::SetChar(register(&arg0)?),
            Rule::bcd => Instruction::BCD(register(&arg0)?),
            Rule::ldmem => Instruction::SetMemPtr(arg0.as_str().parse().unwrap()),
            Rule::setdelay => Instruction::SetDelay(register(&arg0)?),
            Rule::getdelay => Instruction::GetDelay(register(&arg0)?),
            Rule::setsound => Instruction::SetSound(register(&arg0)?),
            Rule::regdmp => Instruction::SetSound(register(&arg0)?),
            Rule::regload => Instruction::RegLoad(register(&arg0)?),
            Rule::setreg => {
                let r1 = pair.into_inner().nth(1).unwrap();
                Instruction::SetReg(register(&arg0)?, register(&r1)?)
            }
            Rule::setimm => {
                let n: u8 = parse_u8(&pair.into_inner().nth(1).unwrap())?;
                Instruction::SetImm(register(&arg0)?, n)
            }
            _ => error(&pair, "unexpected rule")?,
        }),
        None,
    ))
}

fn parse_bytes<'a>(bytes_args: Pair<'a, Rule>) -> ParseResult {
    let hexes = bytes_args.into_inner();
    let mut bytes = Vec::with_capacity(hexes.len());
    for arg in hexes {
        bytes.push(arg.as_str().strip_prefix("0x").unwrap().parse().unwrap());
    }
    Ok((Line::Data(bytes), None))
}

fn parse_instruction<'a>(pair: Pair<'a, Rule>) -> ParseResult {
    match pair.as_rule() {
        Rule::cls => Ok((Line::Instr(Instruction::ClearScreen), None)),
        Rule::ret => Ok((Line::Instr(Instruction::Ret), None)),
        Rule::nop => Ok((Line::Instr(Instruction::Nop), None)),
        Rule::call => parse_call(pair.into_inner()),
        Rule::jump | Rule::jpoff => parse_jump(pair),
        Rule::add
        | Rule::sub
        | Rule::se
        | Rule::sne
        | Rule::or
        | Rule::and
        | Rule::xor
        | Rule::subn
        | Rule::rnd => parse_binop(pair),
        Rule::drw => parse_draw(pair.into_inner()),
        Rule::rsh | Rule::lsh | Rule::skp | Rule::sknp => parse_unop(pair),
        Rule::load => parse_load(pair.into_inner().next().unwrap()),
        Rule::bytes => parse_bytes(pair),
        _ => error(&pair, "unexpected rule")?,
    }
}

pub fn parse_program(file: &str) -> Result<Program, Error> {
    match InstructionParser::parse(Rule::file, file) {
        Ok(file) => {
            let mut program: Program<'_> = Program {
                instructions: vec![],
                labels: HashMap::new(),
                references: vec![],
            };
            for line in file {
                match line.as_rule() {
                    Rule::line => {
                        let label_instr = line.clone().into_inner().next().unwrap();
                        match label_instr.as_rule() {
                            Rule::instruction => {
                                match parse_instruction(label_instr.into_inner().next().unwrap()) {
                                    Ok((instruction, reference)) => {
                                        program.instructions.push(instruction);
                                        program.references.push(reference);
                                    }
                                    Err(e) => return Err(e),
                                }
                            }
                            Rule::label => {
                                program.labels.insert(
                                    label_instr.as_str().strip_suffix(":").unwrap(),
                                    program.instructions.len(),
                                );
                            }
                            rule => {
                                return Err(Error::new_from_pos(
                                    ErrorVariant::CustomError {
                                        message: format!("unexpected rule: {:?}", rule),
                                    },
                                    label_instr.as_span().start_pos(),
                                ))
                            }
                        }
                    }
                    Rule::EOI => {
                        break;
                    }
                    rule => {
                        return Err(Error::new_from_pos(
                            ErrorVariant::CustomError {
                                message: format!("unexpected rule: {:?}", rule),
                            },
                            line.as_span().start_pos(),
                        ))
                    }
                }
            }
            Ok(program)
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use crate::labels::Line;
    use chip8::Instruction;

    #[test]
    fn test_parse1() {
        let instructions = r#"start:
ld v0 1;
ld v1 10;
loop:
add v0 1;
se v0 v1;
jp loop;
end:
jp end;
ld v111 99;
"#;
        let program = super::parse_program(&instructions).unwrap();
        assert_eq!(program.instructions.len(), 7);
        assert_eq!(program.labels.get("start"), Some(&0));
        assert_eq!(program.labels.get("loop"), Some(&2));
        assert_eq!(program.labels.get("end"), Some(&5));
    }

    #[test]
    fn test_parse2() {
        let instructions = r#"start:
ld v0 1;
ld v1 0x10;
loop:
add v0 1;
se v0 v1;
jp loop;
bytes 0x1 0x2 0x3 0x4 0x5;
end:
jp end;
jp start;
"#;
        let mut program = super::parse_program(&instructions).unwrap();
        assert_eq!(program.instructions.len(), 8);
        program.fix_references();
        assert_eq!(program.instructions.len(), 8);
        let last = Line::Instr(Instruction::Jump(0x200));
        assert_eq!(*program.instructions.last().unwrap(), last);
        let intended = Line::Instr(Instruction::Jump(0x20f));
        assert_eq!(program.instructions[6], intended);
    }
}
