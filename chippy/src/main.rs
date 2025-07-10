use cfg_if::cfg_if;
use chippy::driver::{self, run, Chip8Driver, EmulatorMode};
use std::io::Read;

use clap::Parser;
use clio::*;

#[derive(Parser)]
struct Args {
    rom: ClioPath,
    #[arg(short, long)]
    speed: Option<u64>,
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let args: Args = Args::parse();
    let rom_name = args.rom.as_os_str().to_string_lossy().into_owned();
    let mut input = args
        .rom
        .open()
        .expect(&format!("No file named {}", rom_name));
    let mut instructions = Vec::new();
    input
        .read_to_end(&mut instructions)
        .expect(&format!("Failed to read {}", rom_name));
    driver::run(instructions.as_slice(), args.speed, args.debug);
}
