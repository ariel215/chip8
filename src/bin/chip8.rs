use std::io::Read;

use chip8::Emulator;

use clap::Parser;
use clio::*;

#[derive(Parser)]
struct Args{
    rom: ClioPath,
    #[arg(short, long)]
    speed: Option<u64>,
    #[arg(short, long)]
    debug: bool
}

fn main() {
    let args = Args::parse();
    let rom_name = args.rom.as_os_str().to_string_lossy().into_owned();
    let mut input = args.rom.open().expect(&format!("No file named {}", rom_name));
    let mut instructions = Vec::new();
    input.read_to_end(&mut instructions).expect(&format!("Failed to read {}", rom_name ));
    let mut emulator = Emulator::windowed();
    if args.speed.is_some(){
        emulator = emulator.clock_speed(args.speed.unwrap());
    }
    emulator.load_rom(&instructions);
    if args.debug{
        emulator.debug();
        loop{
            if emulator.step(){
                return;
            }
        }
    }
    else {
        emulator.run()
    }
}
