use cfg_if::cfg_if;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::frontend;
use crate::frontend::egui::EguiDisplay;
use crate::frontend::raylib::RaylibDisplay;
use crate::frontend::Chip8Frontend;
use crate::frontend::KeyInput;
use crate::emulator::Chip8;
use crate::frontend::FRAME_DURATION;
use std::process::exit;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub enum EmulatorMode {
    Running,
    Paused,
}

pub trait Chip8Driver{
    fn run(rom: &[u8], speed: Option<u64>, paused: bool);
}

cfg_if!{
    if #[cfg(feature = "egui")] {
        pub fn run(rom: &[u8], speed: Option<u64>, paused: bool){
            <frontend::egui::EguiDriver as Chip8Driver>::run(rom, speed, paused);
        }
    } else {
        pub fn run(rom: &[u8], speed: Option<u64>, paused: bool){
            <frontend::raylib::RaylibDriver as Chip8Driver>::run(rom, speed, paused);
        }
    }
}