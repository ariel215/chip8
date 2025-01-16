use cfg_if::cfg_if;
use chip8::Chip8Driver;
use std::io::Read;

use clap::Parser;
use clio::*;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Parser)]
struct Args {
    rom: ClioPath,
    #[arg(short, long)]
    speed: Option<u64>,
    #[arg(short, long)]
    debug: bool,
}

pub fn main() {
    cfg_if! {
        if #[cfg(target_family = "wasm")] {
            return();
        } else {
            let args = Args::parse();
            let rom_name = args.rom.as_os_str().to_string_lossy().into_owned();
            let mut input = args.rom.open().expect(&format!("No file named {}", rom_name));
            let mut instructions = Vec::new();
            input.read_to_end(&mut instructions).expect(&format!("Failed to read {}", rom_name ));
            let mut driver = Chip8Driver::new(args.speed);
            driver.load_rom(&instructions);
            driver.run()

        }
    }
}
