use std::collections::HashMap;

use clap::builder::Str;
use itertools::Itertools;

use crate::{parser::*};

struct UseDefChain{
    def: usize,
    uses: Vec<usize>
}


fn get_def(statement: &ParseNode) -> Option<&str>{
    match statement {
        ParseNode::Var(v) => Some(v.as_ref()),
        ParseNode::Binary(BinaryOp{operator, left, ..})
            if matches!(operator, Operator::Assign) => {
                get_def(left)
            },
        _ => None
    }
}

fn get_uses<'a>(statement: &'a ParseNode) -> Vec<String> {
    
    fn get_uses_rec(statement: &ParseNode, mut uses: Vec<String>) -> Vec<String>{
        match statement {
            ParseNode::Signed(_) | ParseNode::Unsigned(_) => {uses},
            ParseNode::Var(v) =>{ uses.push(v.clone()); uses}
            ParseNode::Binary(b) => match b.operator {
                Operator::Assign => {get_uses_rec(&b.right,uses )}
                Operator::Plus | Operator::Minus => {
                    let uses = get_uses_rec(&b.left, uses);
                    get_uses_rec(&b.right, uses)
                }
            }
        }
    }
    let uses = Vec::new();
    get_uses_rec(statement, uses)
}


fn use_def_chains(statements: Vec<ParseNode>) -> Result<Vec<Option<UseDefChain>>, String> {
    let mut chains: Vec<Option<UseDefChain>> = statements.iter().map(|_|{None}).collect_vec();
    let mut last_def= HashMap::new();
    for (i, statement) in statements.iter().enumerate() {
        for used in get_uses(statement){
            match last_def.get(&used){
                None => {return Err(format!("undefined variable {}", used));}
                Some(&idx) => {
                    match chains[idx] {
                        None => {return  Err(format!("undefined variable {}", used));}
                        Some::<UseDefChain>(ref mut chain) => {chain.uses.push(i)}
                    }
                }
            }
        }
        if let Some(var) = get_def(statement) {
            chains[i] = Some(UseDefChain{
                def: i,
                uses: Vec::new()
            });
            last_def.insert(var.to_string(), i);
        }
    }
    Ok(chains)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defs(){
        let input = "x=1; y=2; z=3; w = x + y + z;";
        let statements = parse_statements(input).unwrap();
        let defs = statements.iter().map(|s| get_def(s)).collect_vec();
        assert!(defs[0].unwrap() == "x".to_string());
        assert!(defs[1].unwrap() == "y".to_string());
        assert!(defs[2].unwrap() == "z".to_string());

        let uses = statements.iter().map(|s|get_uses(s)).collect_vec();
        assert!(uses[0].len() == 0);
        assert!(uses[1].len() == 0);
        assert!(uses[2].len() == 0);
        assert!(uses[3].len() == 3);
    }


    #[test]
    fn test_chains(){
        let input = "x=1; y=2; z=3; w = x + y + z; x = 2;";
        let statements = parse_statements(input).unwrap();
        let chains = use_def_chains(statements).unwrap();
        for i in 0..3 {
            assert!(chains[i].as_ref().unwrap().def == i);
            assert!(chains[i].as_ref().unwrap().uses.len() == 1);
            assert!(chains[i].as_ref().unwrap().uses[0] == 3);
        }
    }
}