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

struct ParsingError{
    error: chip8::errors::ParseError,
    line_number: usize
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
            println!("Could not read file {}", input_name)
        }
        match assemble(&text) {
            Ok(bytes) => {
                let bytes: Vec<_> = bytes.into_iter().flat_map(u16::to_be_bytes)
                .collect();
                let mut output = args.output.create().expect(&format!("Could not create file {}", output_name));
                output.write_all(&bytes).expect(&format!("Could not write to {}", output_name));
            },
            Err(error) => {
                eprintln!("Error in assembly file {}:{}", input_name, error.line_number+1);
                eprintln!("Could not parse '{}' : {}", error.error.mnemonic, error.error.message)
            }
        }
    }
}


fn assemble(text: &str) -> Result<Vec<u16>,ParsingError>{
    let mut bytes: Vec<u16> = Vec::new();
    for (line_number, line) in text.lines().enumerate(){
        let mnemonics = line.split(";");
        for mnemonic in mnemonics {
            if mnemonic.is_empty() {continue;}
            match Instruction::from_mnemonic(mnemonic.trim()){
                Ok(i) => {bytes.push(i.into())},
                Err(error) => {
                    return Err(ParsingError{
                    error,
                    line_number
                })}
            }
        }
    }
    Ok(bytes)
}
