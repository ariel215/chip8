use std::{array, collections::{BTreeMap, HashMap}, ops::{Range, RangeInclusive}};

use crate::parser::{Operator, ParseNode};
use chip8::Instruction;

// A LongInstruction is just an instruction
// that makes use of a register that may not exist
struct LongInstruction(Instruction);


struct LoweringVisitor<'a> {
    variables: BTreeMap<&'a str, Register>,
    temp_vars: HashMap<String, Register>,
    instructions: Vec<LongInstruction>,
}

#[derive(Debug, Clone, Copy)]
struct Register(u8);

#[derive(Debug)]
enum Value {
    Number(u8),
    Reg(Register),
}

impl Value {
    fn value(&self) -> u8 {
        match self {
            Value::Number(n) => *n,
            Value::Reg(Register(r)) => *r,
        }
    }
}

impl<'a> LoweringVisitor<'a> {
    fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
            temp_vars: HashMap::new(),
            instructions: Vec::new(),
        }
    }

    fn get_or_create_register(&mut self, variable: &'a str) -> Value {
        let next_index = self.variables.len();
        if let Some(reg) = self.variables.get(variable) {
            Value::Reg(*reg)
        } else if let Some(reg) = self.temp_vars.get(variable) {
            Value::Reg(*reg)
        } else {
            Value::Reg(
                *self
                    .variables
                    .entry(variable)
                    .or_insert(Register(next_index as u8)),
            )
        }
    }

    fn create_temp(&mut self) -> Value {
        let count = u8::MAX - (self.temp_vars.len() as u8);
        let name = format!(".temp{}", count);
        self.temp_vars.insert(name, Register(count));
        Value::Reg(Register(count))
    }

    fn lower_node(&mut self, node: &'a ParseNode) -> Option<Value> {
        let result = match node {
            ParseNode::Unsigned(v) => Some(Value::Number(*v)),
            ParseNode::Signed(v) => Some(Value::Number(*v as u8)),
            ParseNode::Var(ref name) => Some(self.get_or_create_register(name.as_str())),
            ParseNode::Binary(operation) => {
                let lhs = self.lower_node(&operation.left).unwrap();
                let rhs = self.lower_node(&operation.right).unwrap();
                match operation.operator {
                    Operator::Assign => {
                        // We've checked in parsing that the left side of an
                        // assignment is always an lvalue (rn just a variable)
                        assert!(matches!(lhs, Value::Reg(_)));
                        let instruction = match rhs {
                            Value::Number(n) => Instruction::SetImm(lhs.value(), n),
                            Value::Reg(vx) => Instruction::SetReg(lhs.value(), vx.0),
                        };
                        self.instructions.push(LongInstruction(instruction));
                        Some(lhs)
                    }
                    Operator::Plus => {
                        let place: Value = match lhs {
                            Value::Number(n) => {
                                let place = self.create_temp();
                                self.instructions
                                    .push(LongInstruction(
                                        Instruction::SetImm(place.value(), n))
                                    );
                                place
                            }
                            Value::Reg(r) => Value::Reg(r),
                        };
                        let instruction = match rhs {
                            Value::Number(nr) => Instruction::AddImm(place.value(), nr),
                            Value::Reg(Register(rr)) => Instruction::AddReg(place.value(), rr),
                        };
                        self.instructions.push(LongInstruction(instruction));
                        Some(place)
                    }

                    Operator::Minus => {
                        let place: Value = match lhs {
                            Value::Number(n) => {
                                let place = self.create_temp();
                                self.instructions
                                    .push(LongInstruction(Instruction::SetImm(place.value(), n)));
                                place
                            }
                            Value::Reg(r) => Value::Reg(r),
                        };
                        let instruction = match rhs {
                            Value::Number(nr) => {
                                let subtractend = self.create_temp().value();
                                self.instructions.push(LongInstruction(
                                    Instruction::SetImm(subtractend, nr))
                                );
                                Instruction::SubReg(place.value(), subtractend)
                            }
                            Value::Reg(Register(rr)) => Instruction::SubReg(place.value(), rr),
                        };
                        self.instructions.push(LongInstruction(instruction));
                        Some(place)
                    }
                    Operator::Times | Operator::Divided=> {
                        todo!("these both need to be implemented in software")
                    }
                    _ => todo!()
                }
            }
        };
        println!("visiting {:?}; got {:?}", node, result);
        result
    }
}

pub fn lower_ast(statements: &[ParseNode]) -> Vec<LongInstruction> {
    /// Produces a vector of pseudoinstructions from ParseNodes
    /// The reason these are not valid instructions is that 
    /// there are 256 valid registers instead of 16
    let mut visitor = LoweringVisitor::new();
    for statement in statements.iter() {
        visitor.lower_node(statement);
    }
    visitor.instructions
}

/// Find the first place a given register is defined
/// Returns: A map from instruction number to register number
pub fn defs<'a>(statements: impl Iterator<Item =  &'a LongInstruction>) -> Vec<Vec<usize>>{
    statements.map(|instr| {
        match instr.0 {
            // Instructions with a single register operand
                Instruction::SetImm(r, _) | 
                Instruction::AddImm(r, _) | 
                Instruction::GetDelay(r) | 
                Instruction::WaitForKey(r) | 
                Instruction::Rsh(r) |
                Instruction::Lsh(r) => {
                vec![r as usize]
            }, 
            // Instructions with two register operands
            Instruction::SetReg(r1, r2) |
            Instruction::OrReg(r1, r2) |
            Instruction::AndReg(r1,r2) |
            Instruction::XorReg(r1,r2) |
            Instruction::AddReg(r1,r2) |
            Instruction::SubReg(r1,r2) |
            Instruction::SubFrom(r1,r2) => {
                vec![r1 as usize, r2 as usize]
            }
            // Special Cases
            Instruction::RegLoad(r) => {
                (0..r as usize).into_iter().collect()
            }
            _ => {Vec::new()}
    }
}).collect()
}

fn swap(vx: u8, vy: u8) -> Vec<Instruction>{
    vec![
        Instruction::SetMemPtr(0x100),
        Instruction::RegDump(vx),
        Instruction::SetMemPtr(0x100 + vy),
        Instruction::RegLoad(0),
        Instruction::SetReg(vx, 0),
        Instruction::SetMemPtr(0x100),
        Instruction::RegLoad(0)
    ]
}


fn lower(linstr: &LongInstruction) -> Vec<Instruction> {
    let i = linstr.0;
    match linstr.0 {
        // These instructions use no registers
        Instruction::ClearScreen |
        Instruction::Ret |
        Instruction::Nop |
        Instruction::Jump(_) |
        Instruction::Call(_) |
        Instruction::SetMemPtr(_) => vec![i],
        // These instructions use a single register
        Instruction::SkipEqImm(r, _) |
        Instruction::SkipNeImm(r, _) |
        Instruction::SetImm(r, _) |
        Instruction::AddImm(r, _) |
        Instruction::Rsh(r) |
        Instruction::Lsh(r) |
        Instruction::Rand(r, _) |hi
        Instruction::GetDelay(r) |
        Instruction::WaitForKey(r) |
        Instruction::SetDelay(r) |
        Instruction::SetSound(r)| 
        Instruction::SetChar(r) |
        Instruction::BCD(r)  => {
            if r > 14 {
                let mut instrs = swap(0,r);
                let new_instr: u16 = i.into() & 0xf0ff;
                instrs.push(new_instr.into());
                instrs
            }
        }
        // These instructions use 2 registers
        Instruction::SkipEqReg(vx,vy) |
        Instruction::SetReg(vx,vy) |
        Instruction::OrReg(vx,vy) |
        Instruction::AndReg(vx,vy) |
        Instruction::XorReg(vx,vy) |
        Instruction::AddReg(vx,vy) |
        Instruction::SubReg(vx,vy) |
        Instruction::SubFrom(vx,vy) |
        Instruction::SkipNeReg(vx,vy) |
        Instruction::Draw(vx,vy, _) => {
            let mut instrs = Vec::new();
            let xnew = 
            if vx > 14 {
                instrs.extend_from_slice(&swap(0,vx));
                0
            } else { vx };
            let ynew = 
                if vy > 14 {
                    instrs.extend_from_slice(&swap(1, vy));
                    1
                } else  { vy };
            let new_instr: u16 = i.into() & 0xf00f | (xnew << 8 ) | (ynew << 4);
            instrs.push(new_instr.into())
        }
        /// These instructions don't work with extended registers
        Instruction::SkipKeyPressed(_) |
        Instruction::SkipKeyNotPressed(_) |
        Instruction::JumpOffset(_) |
        Instruction::AddMemPtr(_) |
        Instruction::RegDump(_) |
        Instruction::RegLoad(_) => vec![i]
    }
}



// pub fn uses<'a>(statements: impl Iterator<Item = &'a LongInstruction>) -> Vec<Vec<usize>> {
//     statements.map(|instr| {
//         match instr.0 {
//             // Instructions with a single register operand
//             Instruction::SkipEqImm(r, _) |
//                 Instruction::SkipNeImm(r, _) |
//                 Instruction::SetImm(r, _) | 
//                 Instruction::AddImm(r, _) | 
//                 Instruction::Rand(r, _) | 
//                 Instruction::SkipKeyPressed(r) | 
//                 Instruction::SkipKeyNotPressed(r) | 
//                 Instruction::GetDelay(r) | 
//                 Instruction::WaitForKey(r) | 
//                 Instruction::SetDelay(r) | 
//                 Instruction::SetSound(r) | 
//                 Instruction::AddMemPtr(r) | 
//                 Instruction::SetChar(r) | 
//                 Instruction::BCD(r)| 
//                 Instruction::Rsh(r) |
//                 Instruction::Lsh(r) => {
//                 vec![r as usize]
//             }, 
//             // Instructions with two register operands
//             Instruction::SkipEqReg(r1, r2) |
//             Instruction::SetReg(r1, r2) |
//             Instruction::OrReg(r1, r2) |
//             Instruction::AndReg(r1,r2) |
//             Instruction::XorReg(r1,r2) |
//             Instruction::AddReg(r1,r2) |
//             Instruction::SubReg(r1,r2) |
//             Instruction::SubFrom(r1,r2) |
//             Instruction::SkipNeReg(r1,r2) |
//             Instruction::Draw(r1,r2, _)  => {
//                 vec![r1 as usize, r2 as usize]
//             }
//             // Special Cases
//             Instruction::JumpOffset(_) => {vec![0]},
//             Instruction::RegDump(r) => {
//                 (0..=r as usize).into_iter().collect()
//             }
//             // Instructions with no register uses
//             Instruction::RegLoad(_) | 
//             Instruction::ClearScreen |
//             Instruction::Ret |
//             Instruction::Nop |
//             Instruction::Jump(_) |
//             Instruction::Call(_) |
//             Instruction::SetMemPtr(_) => {Vec::new()},
//         }
//     }).collect()
// }

// fn last_use<'a>(statements: impl Iterator<Item = &'a LongInstruction>) -> [Option<usize>; 256]{
//     let use_vec = uses(statements);
//     let mut ends = [None; 256];
//     for (i,use_list) in use_vec.iter().enumerate().rev(){
//         for &reg in use_list {
//             ends[reg].get_or_insert(i);
//         }
//     }
//     ends
// }


// #[derive(Debug, Clone, Copy)]
// enum RegisterAssignment{
//     Reg(u8),
//     Spill
// }

// impl RegisterAssignment {
//     fn reg(val: u8) -> Self{
//         if val > 0xf {
//             panic!()
//         }
//         RegisterAssignment::Reg(val)
//     }
// }


// fn alloc(instrs: &[LongInstruction]) -> [Option<RegisterAssignment>; 256]{
//     let mut free = [true; 16];
//     let mut assignments = [const {None}; 256];
//     let mut seen = [false; 256];
//     let live_ends = last_use(instrs.iter());
//     for (i,(d,u)) in defs(instrs.iter()).iter().zip(uses(instrs.iter()).iter()).enumerate(){
//         for &r in d.iter() {
//             if !seen[r] {
//                 match free.iter().enumerate().find(|&x| *x.1){
//                     Some((reg,_)) => {
//                         free[reg] = false;
//                         assignments[r] = Some(RegisterAssignment::reg(reg.try_into().unwrap()))
//                     }
//                     None => assignments[r] = Some(RegisterAssignment::Spill)
//                 }
//             }
//             seen[r] = true
//         }
//         for &use_ in u {
//             if live_ends[use_] == Some(i) {
//                 free[use_] = true
//             }
//         }
//     }
//     assignments
// }

