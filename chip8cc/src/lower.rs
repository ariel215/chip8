use crate::parser::*;
use chip8::Instruction;

pub fn lower_node(node: &ParseNode) -> Vec<Instruction>{
    match node {
        ParseNode::Unsigned(v) => vec![Instruction::SetImm(0, *v)],
        ParseNode::Signed(v) => {
            let u = v.to_be_bytes();
            vec![Instruction::SetImm(0, u[0])]
        }
    }
}
