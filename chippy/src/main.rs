use cfg_if::cfg_if;
use chippy::driver::{self, run, Chip8Driver, EmulatorMode};
use std::io::Read;

use clap::Parser;
use clio::*;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    speed: Option<u64>,
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let args = Args::parse();
    driver::run(args.speed, args.debug);
}
