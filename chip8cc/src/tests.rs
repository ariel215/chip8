use crate::lower::lower;
use crate::parser;
use crate::lower;
use crate::parser::parse_statements;
use anyhow;
use chip8::instructions;
use itertools::PadUsing;


#[test]
fn test_assign(){
    let input = "x = 1;\n";
    let (_, nodes) = parse_statements(input).unwrap();
    let instructions = lower(&nodes);
    assert!(instructions.len() == 1);
    assert!(matches!(instructions[0], chip8::Instruction::SetImm(_,_)))
}


#[test]
fn test_multi_assign(){
    let input = "x=1;y=3;z=y;";
    let (_, nodes) = parse_statements(input).unwrap();
    assert!(nodes.len() == 3, "{:?}", nodes);
    let instructions = lower(&nodes);
    assert!(instructions.len() == 3)
}

#[test]
fn test_add_1(){
    let input = "x = 1 + 2;"; // in preorder: (= x (+ 1 2))
    let (_, nodes) = parse_statements(input).unwrap();
    assert!(nodes.len() == 1);
    let instructions = lower(&nodes);
    assert!(instructions.len() == 3);
    println!("result: {:?}", instructions);
}


#[test]
fn test_add_multiple(){
    let input = "x =  1 + 2 - 3 + z;";
    let (_, nodes) = parse_statements(input).unwrap();
    let instructions = lower(&nodes);
    dbg!(&instructions);
    assert!(instructions.len() == 6);
}
