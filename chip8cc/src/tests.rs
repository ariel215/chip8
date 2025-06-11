use crate::lower;
use crate::lower::lower_ast;
use crate::parser;
use crate::parser::parse_statements;
use chip8::instructions;

#[test]
fn test_assign() {
    let input = "x = 1;\n";
    let nodes = parse_statements(input).unwrap();
    let instructions = lower_ast(&nodes);
    assert!(instructions.len() == 1);
    assert!(matches!(instructions[0], chip8::Instruction::SetImm(_, _)))
}

#[test]
fn test_multi_assign() {
    let input = "x=1;y=3;z=y;";
    let nodes = parse_statements(input).unwrap();
    assert!(nodes.len() == 3, "{:?}", nodes);
    let instructions = lower_ast(&nodes);
    assert!(instructions.len() == 3)
}

#[test]
fn test_add_1() {
    let input = "x = 1 + 2;"; // in preorder: (= x (+ 1 2))
    let nodes = parse_statements(input).unwrap();
    assert!(nodes.len() == 1);
    let instructions = lower_ast(&nodes);
    assert!(instructions.len() == 3);
    println!("result: {:?}", instructions);
}

#[test]
fn test_add_multiple() {
    let input = "x =  1 + 2 - 3 + z;";
    let nodes = parse_statements(input).unwrap();
    let instructions = lower_ast(&nodes);
    dbg!(&instructions);
    assert!(instructions.len() == 6);
}
