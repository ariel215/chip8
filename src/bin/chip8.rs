use cfg_if::cfg_if;
use chip8::driver;
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
    #[arg(short, long)]
    paused: bool,
}

pub fn main() {
    cfg_if! {
        if #[cfg(target_family = "wasm")] {
            driver::run(&[], None, true)
        } else {
            let args = Args::parse();
            let rom_name = args.rom.as_os_str().to_string_lossy().into_owned();
            let mut input = args.rom.open().expect(&format!("No file named {}", rom_name));
            let mut instructions = Vec::new();
            input.read_to_end(&mut instructions).expect(&format!("Failed to read {}", rom_name ));
            driver::run(&instructions, args.speed, args.paused);
        }
    }
}
