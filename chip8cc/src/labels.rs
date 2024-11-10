use std::{collections::HashMap, hash::Hash};

use chip8::Instruction;
use pest::{error::ErrorVariant, iterators::{Pair, Pairs}, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/labels.pest"]
pub struct InstructionParser;
pub type Error = pest::error::Error<Rule>;
pub type ParseResult<'a> = Result<(Instruction, Option<&'a str>), Error>;

pub struct Program<'a>{
    instructions: Vec<Instruction>,
    labels: HashMap<&'a str, usize>,
    references: Vec<Option<&'a str>>
}

impl<'a> Program<'a> {
    pub fn fix_references(&mut self){
        for label in self.references.iter(){
            if let Some(label) = *label{
                let addr = self.labels[label];
                let new_addr = addr * chip8::INSTRUCTION_SIZE + 0x200;
                let new_instruction = match self.instructions[addr] {
                    Instruction::Call(_) => Instruction::Call(new_addr as u16),
                    Instruction::Jump(_) => Instruction::Jump(new_addr as u16),
                    _ => panic!("unexpected instruction: {}", self.instructions[addr])
                };
                self.instructions[addr] = new_instruction;
            }
        }
    }

    pub fn compile(&self) -> Vec<u8>{
        self.instructions.iter()
            .map(|i| {<Instruction as Into<u16>>::into(*i)}).flat_map(u16::to_be_bytes)
            .collect()
    }
}


fn register<'a>(name: &Pair<'a, Rule>) -> Result<u8,Error> {
    let s  = name.as_str();
    if let Some(reg) = s.to_ascii_lowercase().strip_prefix('v'){
        reg.parse().or(Err(Error::new_from_span(
            ErrorVariant::CustomError { message: "invalid register name".to_string() }, name.as_span())))
    } else {
        Err(Error::new_from_span(
            ErrorVariant::CustomError { message: "invalid register name".to_string() }, name.as_span()))
    }
}

fn bad_rule_error<'a, T>(rule: &Pair<'a, Rule>) -> Result<T, Error>{
        return Err(Error::new_from_pos(
            ErrorVariant::CustomError { message: format!("unexpected rule: {:?}", rule) }, 
            rule.as_span().start_pos())
    )
}

fn parse_binop<'a>(pair: Pair<'a, Rule>) -> Result<(Instruction, Option<&'a str>), Error>{
    let rule = pair.as_rule();
    let mut parts = pair.into_inner();
    if matches!(rule, Rule::add){
        if let Some(ref reg) = parts.find_first_tagged("addi"){
            return Ok((Instruction::AddMemPtr(register(reg)?),None))
        } 
    }
    let r1 = register(&parts.next().unwrap())?;
    let p2 = &parts.next().unwrap();
    let r2 = match p2.as_rule() {
        Rule::register => register(p2)?,
        Rule::number => p2.as_str().parse().unwrap(),
        _ => bad_rule_error(p2)?
    };
    Ok((match rule {
        Rule::add => {if matches!(p2.as_rule(), Rule::number) {Instruction::AddImm(r1, r2)} else {Instruction::AddReg(r1, r2)}},
        Rule::se => {if matches!(p2.as_rule(), Rule::number) {Instruction::SkipEqImm(r1, r2)} else {Instruction::SkipEqReg(r1, r2)}},
        Rule::sne => {if matches!(p2.as_rule(), Rule::number) {Instruction::SkipNeImm(r1, r2)} else {Instruction::SkipNeReg(r1, r2)}},
        Rule::sub => {Instruction::SubReg(r1, r2)}
        Rule::or => {Instruction::OrReg(r1, r2)},
        Rule::and => {Instruction::AndReg(r1, r2)},
        Rule::xor => {Instruction::XorReg(r1, r2)},
        Rule::subn => {Instruction::SubFrom(r1, r2)}
        _ => {bad_rule_error(p2)?}
    }, None))
}

fn parse_addr<'a>(addr: Pairs<'a, Rule>) -> (u16, Option<&'a str>){
    if let Some(fixed_addr) = addr.find_first_tagged("fixed"){
        let fixed_addr = fixed_addr.as_str().parse().unwrap();
        return (fixed_addr, None)
    }
    if let Some(label) = addr.find_first_tagged("label"){
        return (0, Some(label.as_str()))
    }
    unreachable!()
}

fn parse_call<'a>(mut call: Pairs<'a, Rule>) -> (Instruction, Option<&'a str>){
    let addr = call.nth(1).unwrap().into_inner();
    let addr = parse_addr(addr);
    return (Instruction::Call(addr.0), addr.1)
}

fn parse_jump<'a>(jump: Pair<'a, Rule>) -> Result<(Instruction, Option<&'a str>), Error>{
    let rule = jump.as_rule();
    let mut jump_in = jump.clone().into_inner();
    let addr = jump_in.next().unwrap().into_inner();
    let addr = parse_addr(addr);
    Ok((match rule {
        Rule::jump => Instruction::Jump(addr.0),
        Rule::jpoff => Instruction::JumpOffset(addr.0),
        _ => {bad_rule_error(&jump)?}
    }, addr.1))
}

fn parse_draw<'a>(mut draw: Pairs<'a, Rule>) -> Result<(Instruction, Option<&'a str>), Error>{
    let r1 = register(&draw.nth(1).unwrap())?;
    let r2 = register(&draw.nth(1).unwrap())?;
    let n = draw.nth(1).unwrap().as_str().parse().unwrap();
    return Ok((Instruction::Draw(r1, r2, n), None))
}


fn parse_unop<'a>(op: Pair<'a, Rule>) -> Result<(Instruction, Option<&'a str>), Error>{
    let rule = op.as_rule();
    let reg = register(&op.clone().into_inner().nth(1).unwrap())?;
    Ok((match rule {
        Rule::rsh => Instruction::Rsh(reg),
        Rule::lsh => Instruction::Lsh(reg),
        Rule::skp => Instruction::SkipKeyPressed(reg),
        Rule::sknp => Instruction::SkipKeyNotPressed(reg),
        _ => {bad_rule_error(&op)?}
    }, None))
}

fn parse_load<'a>(load_args: Pair<'a, Rule>) -> ParseResult<'a>{
    let pair = load_args.into_inner().next().unwrap();
    let arg0 = pair.clone().into_inner().next().unwrap();
    Ok((match pair.as_rule() {
        Rule::ldchar => {
            Instruction::SetChar(register(&arg0)?)
        }
        Rule::bcd => {
            Instruction::BCD(register(&arg0)?)
        }
        Rule::ldmem => {
            Instruction::SetMemPtr(arg0.as_str().parse().unwrap())
        }
        Rule::setdelay => {
            Instruction::SetDelay(register(&arg0)?)
        }
        Rule::getdelay => Instruction::GetDelay(register(&arg0)?),
        Rule::setsound => Instruction::SetSound(register(&arg0)?),
        Rule::regdmp => Instruction::SetSound(register(&arg0)?),
        Rule::regload => Instruction::RegLoad(register(&arg0)?),
        Rule::setreg => {
            let r1 = pair.into_inner().nth(1).unwrap();
            Instruction::SetReg(register(&arg0)?, register(&r1)?)
        }
        Rule::setimm => {
            let n: u8 = pair.into_inner().nth(1).unwrap().as_str().parse().unwrap();
            Instruction::SetImm(register(&arg0)?, n)
        }
        _ => bad_rule_error(&pair)?
    }, None))
}

pub fn parse_instruction<'a>(pair: Pair<'a, Rule>) -> Result<(Instruction, Option<&'a str>),Error>{
    match pair.as_rule(){
        Rule::cls => { Ok((Instruction::ClearScreen, None))}
        Rule::ret => {Ok((Instruction::Ret, None))}
        Rule::nop => { Ok((Instruction::Nop, None))}
        Rule::call => { Ok(parse_call(pair.into_inner()))}
        Rule::jump | Rule::jpoff => { Ok(parse_jump(pair)?)}
        Rule::add | Rule::sub | Rule::se | Rule::sne 
            | Rule::or | Rule::and | Rule::xor | Rule::subn
            | Rule::rnd => {parse_binop(pair)},
        Rule::drw => { parse_draw(pair.into_inner())},
        Rule::rsh | Rule::lsh | Rule::skp | Rule::sknp => {parse_unop(pair)},
        Rule::load => {parse_load(pair.into_inner().next().unwrap())}
        _ => {bad_rule_error(&pair)?}
    }
}


pub fn parse_program<'a>(file:&'a str)->Result<Program<'_>, Error>{
   match InstructionParser::parse(Rule::file, file){
        Ok(file) => {
            let mut program: Program<'_> = Program{
                instructions: vec![],
                labels: HashMap::new(),
                references: vec![]
            };

            for line in file{
                match line.as_rule(){
                    Rule::line => {
                        let label_instr = line.clone().into_inner().next().unwrap();
                        match label_instr.as_rule(){
                            Rule::instruction => {
                                match parse_instruction(label_instr.into_inner().next().unwrap()){
                                    Ok((instruction, reference)) => {
                                        program.instructions.push(instruction);
                                        program.references.push(reference);
                                    },
                                    Err(e) => {return Err(e)}
                                }
                            }
                            Rule::label => {
                                program.labels.insert(label_instr.as_str().strip_suffix(":").unwrap(),
                                    program.instructions.len()
                            );},
                            rule =>                     return Err(Error::new_from_pos(
                                ErrorVariant::CustomError { message: format!("unexpected rule: {:?}", rule) }, 
                                label_instr.as_span().start_pos()))
        
                    }
                },
                Rule::EOI => {break;}
                rule => { return Err(Error::new_from_pos(
                    ErrorVariant::CustomError { message: format!("unexpected rule: {:?}", rule) }, 
                    line.as_span().start_pos()))
                }
            }
        }
        return Ok(program)
    },
    Err(err) => Err(err)
    }
}

#[cfg(test)]
mod tests {
    use pest::Parser;

    
    #[test]
    fn test_parse1(){
        let instructions = 
r#"start:
ld v0 1;
ld v1 10;
loop:
add v0 1;
se v0 v1;
jp loop;
end:
jp end;
"#;
        let program = super::parse_program(&instructions).unwrap();
        assert_eq!( program.instructions.len(), 6);
        assert_eq!(program.labels.get("start"), Some(&0));
        assert_eq!(program.labels.get("loop"), Some(&2));
        assert_eq!(program.labels.get("end"), Some(&5));
    }
}