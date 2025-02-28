use std::marker::PhantomData;

use crate::labels::Line;
use chip8::Instruction;

#[derive(Clone, Copy)]
struct InstructionHandle(usize);

#[derive(Clone, Copy)]
struct BBHandle(usize);

struct BasicBlock{
    start: InstructionHandle,
    end: InstructionHandle, 
}

impl BasicBlock{
    fn blocks(instructions: &[Line]) -> Vec<BasicBlock> {
        let mut blocks = vec![];
        let mut maybe_start = None;
        let mut maybe_end = None;
        for (i,line) in instructions.iter().enumerate(){
            maybe_start = maybe_start.or( Some(InstructionHandle(i)));
            match line {
                Line::Instr(instr)=> {
                    match instr {
                        Instruction::Call(_)
                        | Instruction::Jump(_)
                        | Instruction::JumpOffset(_)
                        | Instruction::SkipEqImm(_,_ )
                        | Instruction::SkipEqReg(_,_ )
                        | Instruction::SkipKeyNotPressed(_)
                        | Instruction::SkipKeyPressed(_)
                        | Instruction::SkipNeImm(_,_ )
                        | Instruction::SkipNeReg(_,_) => {
                            maybe_end = Some(InstructionHandle(i))
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            if let (Some(_), Some(_)) = (&maybe_start, &maybe_end){
                let start = maybe_start.take().unwrap();
                let end = maybe_end.take().unwrap();
                blocks.push(BasicBlock{start, end});
            } 
        }
        return blocks
    }
} 


struct CFG {
    blocks: Vec<BasicBlock>,
    edges: Vec<Vec<BBHandle>>
}

impl CFG {
    fn new(instructions: &[Instruction]) -> Self {
        todo!()
    }
}