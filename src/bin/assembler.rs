use std::io::{Read, Write};
use std::result::Result;

use chip8::Instruction;
use clap::Parser;
use clio::*;
use itertools::{self, Itertools};


#[derive(Parser)]
struct Args{
    input: ClioPath,
    output: ClioPath,
    #[arg(short, long,)]
    disassemble: bool
}

struct ParsingError{
    error: chip8::errors::ParseError,
    line_number: usize
}

fn main(){
    let args = Args::parse();
    if args.disassemble{
        disassemble(args.input, args.output)
    } else {
        assemble(args.input, args.output)
    }
}

fn disassemble(input: ClioPath, output: ClioPath){
    let input_name = input.file_name().map_or("stdin".to_owned(), 
    |name| name.to_string_lossy().into_owned());
    let output_name = input.file_name().map_or("stdout".to_owned(), 
    |name| name.to_string_lossy().into_owned());
    let mut input = input.open().expect(&format!("Could not open {input_name}"));
    let mut bytes: Vec<u8> = Vec::new();
    if input.read_to_end(&mut bytes).is_err(){
        println!("Error reading {}", input_name)
    }
    // assert!(bytes.len() % 2 == 0);
    let instrs = bytes.iter().tuples().map(
        |(upper,lower)| u16::from_be_bytes([*upper, *lower]).into()
    ).take_while(|i| !matches!(*i,Instruction::Nop)).collect::<Vec<Instruction>>();
    let mut mnemonics = instrs.iter().map(|i| i.to_string());
    let mut output = output.create().expect(&format!("Could not create file {}", output_name));
    output.write(mnemonics.join(";\n").as_bytes())
        .expect(&format!("could not write to file {output_name}"));
}



fn assemble(input: ClioPath, output: ClioPath){
    let input_name = input.file_name().map_or("stdin".to_owned(), 
    |name| name.to_string_lossy().into_owned());
    let output_name = input.file_name().map_or("stdout".to_owned(), 
    |name| name.to_string_lossy().into_owned());
    if let Ok(ref mut input) = input.open(){
        let mut text = String::new();
        if input.read_to_string(&mut text).is_err(){
            println!("Could not read file {}", input_name)
        }
        match to_binary(&text) {
            Ok(bytes) => {
                let bytes: Vec<_> = bytes.into_iter().flat_map(u16::to_be_bytes)
                .collect();
                let mut output = output.create().expect(&format!("Could not create file {}", output_name));
                output.write_all(&bytes).expect(&format!("Could not write to {}", output_name));
            },
            Err(error) => {
                eprintln!("Error in assembly file {}:{}", input_name, error.line_number+1);
                eprintln!("Could not parse '{}' : {}", error.error.mnemonic, error.error.message);
            }
        }
    } else {
        eprintln!("Could not open file {input_name}");
    }
}

fn to_binary(text: &str) -> Result<Vec<u16>, ParsingError>{
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

