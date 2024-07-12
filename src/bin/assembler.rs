use std::io::{Read, Write};
use std::result::Result;

use chip8::Instruction;
use clap::Parser;
use clio::*;

#[derive(Parser)]
struct Args{
    input: ClioPath,
    output: ClioPath
}

fn main(){
    let args = Args::parse();
    let input_name = args.input.file_name().map_or("stdin".to_owned(), 
    |name| name.to_string_lossy().into_owned());
    let output_name = args.input.file_name().map_or("stdin".to_owned(), 
    |name| name.to_string_lossy().into_owned());
    if let Ok(ref mut input) = args.input.open(){
        let mut text = String::new();
        if input.read_to_string(&mut text).is_err(){
            print!("Could not read file {}", input_name)
        }
        if let Ok(bytes) = assemble(&text){
            let bytes: Vec<_> = bytes.into_iter().flat_map(u16::to_be_bytes)
            .collect();
            let mut output = args.output.create().expect(&format!("Could not create {}", output_name));
            output.write_all(&bytes).expect(&format!("Could not write to {}", output_name));
        }
    }
}


fn assemble(text: &str) -> Result<Vec<u16>,chip8::errors::ParseError>{
    let mnemonics = text.split(";");
    let mut bytes: Vec<u16> = Vec::new();
    for mnemonic in mnemonics {
        bytes.push(Instruction::from_mnemonic(mnemonic)?.into());
    }
    Ok(bytes)
}