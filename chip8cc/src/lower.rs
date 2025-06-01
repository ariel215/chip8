use std::collections::{BTreeMap, HashMap};

use crate::parser::{Operator, ParseNode};
use chip8::Instruction;

struct LoweringVisitor<'a> {
    variables: BTreeMap<&'a str, Register>,
    temp_vars: HashMap<String, Register>,
    instructions: Vec<Instruction>,
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
                        self.instructions.push(instruction);
                        Some(lhs)
                    }
                    Operator::Plus => {
                        let place: Value = match lhs {
                            Value::Number(n) => {
                                let place = self.create_temp();
                                self.instructions
                                    .push(Instruction::SetImm(place.value(), n));
                                place
                            }
                            Value::Reg(r) => Value::Reg(r),
                        };
                        let instruction = match rhs {
                            Value::Number(nr) => Instruction::AddImm(place.value(), nr),
                            Value::Reg(Register(rr)) => Instruction::AddReg(place.value(), rr),
                        };
                        self.instructions.push(instruction);
                        Some(place)
                    }

                    Operator::Minus => {
                        let place: Value = match lhs {
                            Value::Number(n) => {
                                let place = self.create_temp();
                                self.instructions
                                    .push(Instruction::SetImm(place.value(), n));
                                place
                            }
                            Value::Reg(r) => Value::Reg(r),
                        };
                        let instruction = match rhs {
                            Value::Number(nr) => {
                                let subtractend = self.create_temp().value();
                                self.instructions.push(Instruction::SetImm(subtractend, nr));
                                Instruction::SubReg(place.value(), subtractend)
                            }
                            Value::Reg(Register(rr)) => Instruction::SubReg(place.value(), rr),
                        };
                        self.instructions.push(instruction);
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

pub fn lower(statements: &[ParseNode]) -> Vec<Instruction> {
    let mut visitor = LoweringVisitor::new();
    for statement in statements.iter() {
        visitor.lower_node(statement);
    }
    visitor.instructions
}
