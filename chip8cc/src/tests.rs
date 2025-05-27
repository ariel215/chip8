use crate::parser;
use crate::lower;
use anyhow;
use itertools::PadUsing;

#[test]
fn test_number(){
    let (_, parse_result) = parser::parse_number("100;").unwrap();
    let instructions = lower::lower_node(&parse_result);
    assert!(instructions.len() == 1)

}
