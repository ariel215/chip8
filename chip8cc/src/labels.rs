use std::{collections::HashMap, hash::Hash};

use chip8::Instruction;
use pest::{iterators::{Pair, Pairs}, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/labels.pest"]
pub struct InstructionParser;
pub type Error = pest::error::Error<Rule>;



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
                    _ => unreachable!()
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


fn register(name: &str) -> u8 {
    return name.to_ascii_lowercase().strip_prefix('v').unwrap().parse().unwrap()
}

fn parse_binop<'a>(pair: Pair<'a, Rule>) -> (Instruction, Option<&'a str>){
    let rule = pair.as_rule();
    let mut parts = pair.into_inner();
    if matches!(rule, Rule::add){
        if let Some(reg) = parts.find_first_tagged("addi"){
            return (Instruction::AddMemPtr(register(reg.as_str())), None)
        }
    }
    let r1 = register(parts.nth(1).unwrap().as_str());
    let p2 = parts.nth(1).unwrap();
    let r2 = match p2.as_rule() {
        Rule::register => register(p2.as_str()),
        Rule::number => p2.as_str().parse().unwrap(),
        _ => unreachable!()
    };
    (match rule {
        Rule::add => {if matches!(p2.as_rule(), Rule::number) {Instruction::AddImm(r1, r2)} else {Instruction::AddReg(r1, r2)}},
        Rule::se => {if matches!(p2.as_rule(), Rule::number) {Instruction::SkipEqImm(r1, r2)} else {Instruction::SkipEqReg(r1, r2)}},
        Rule::sne => {if matches!(p2.as_rule(), Rule::number) {Instruction::SkipNeImm(r1, r2)} else {Instruction::SkipNeReg(r1, r2)}},
        Rule::sub => {Instruction::SubReg(r1, r2)}
        Rule::or => {Instruction::OrReg(r1, r2)},
        Rule::and => {Instruction::AndReg(r1, r2)},
        Rule::xor => {Instruction::XorReg(r1, r2)},
        Rule::subn => {Instruction::SubFrom(r1, r2)}
        _ => {unreachable!()}
    }, None)

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

fn parse_jump<'a>(mut jump: Pair<'a, Rule>) -> (Instruction, Option<&'a str>){
    let rule = jump.as_rule();
    let mut jump = jump.into_inner();
    let addr = jump.nth(1).unwrap().into_inner();
    let addr = parse_addr(addr);
    (match rule {
        Rule::jump => Instruction::Jump(addr.0),
        Rule::jpoff => Instruction::JumpOffset(addr.0),
        _ => unreachable!()
    }, addr.1)
}

fn parse_draw<'a>(mut draw: Pairs<'a, Rule>) -> (Instruction, Option<&'a str>){
    let r1 = register(draw.nth(1).unwrap().as_str());
    let r2 = register(draw.nth(1).unwrap().as_str());
    let n = draw.nth(1).unwrap().as_str().parse().unwrap();
    return (Instruction::Draw(r1, r2, n), None)
}


fn parse_unop<'a>(op: Pair<'a, Rule>) -> (Instruction, Option<&'a str>){
    let rule = op.as_rule();
    let reg = register(op.into_inner().nth(1).unwrap().as_str());
    (match rule {
        Rule::rsh => Instruction::Rsh(reg),
        Rule::lsh => Instruction::Lsh(reg),
        Rule::skp => Instruction::SkipKeyPressed(reg),
        Rule::sknp => Instruction::SkipKeyNotPressed(reg),
        _ => unreachable!()
    }, None)
}

pub fn parse_instruction<'a>(pair: Pair<'a, Rule>) -> (Instruction, Option<&'a str>){
    match pair.as_rule(){
        Rule::cls => { (Instruction::ClearScreen, None)}
        Rule::ret => { (Instruction::Ret, None)}
        Rule::nop => { (Instruction::Nop, None)}
        Rule::call => { parse_call(pair.into_inner())}
        Rule::jump | Rule::jpoff => { parse_jump(pair)}
        Rule::add | Rule::sub | Rule::se | Rule::sne 
            | Rule::or | Rule::and | Rule::xor | Rule::subn
            | Rule::rnd => {parse_binop(pair)},
        Rule::drw => { parse_draw(pair.into_inner())},
        Rule::rsh | Rule::lsh | Rule::skp | Rule::sknp => {parse_unop(pair)},
        _ => unreachable!()
    }
}


pub fn parse_program<'a>(file:&'a str)->Result<Program<'_>, Error>{
   match InstructionParser::parse(Rule::file, file){
        Ok(mut file) => {
        let file = file.next().unwrap();
        let mut program = Program{
            instructions: vec![],
            labels: HashMap::new(),
            references: vec![]
        };

        for line in file.into_inner(){
            match line.as_rule(){
                Rule::instruction => {
                    let (instruction, reference) = parse_instruction(line.into_inner().next().unwrap());
                    program.instructions.push(instruction);
                    program.references.push(reference);
                }
                Rule::label => {
                    program.labels.insert(line.as_str().strip_suffix(":").unwrap(),
                        program.instructions.len()
                );
                }
                Rule::EOI => {break;}
                _ => unreachable!()
            }
        }
        return Ok(program)
    },
    Err(err) => Err(err)
    }
}