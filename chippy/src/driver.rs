use cfg_if::cfg_if;
#[cfg(any(feature = "egui", target_family = "wasm"))]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::frontend;

#[derive(Clone, Copy)]
#[cfg_attr(any(feature="egui", target_family="wasm"), wasm_bindgen)]
pub enum EmulatorMode {
    Running,
    Paused,
}

pub trait Chip8Driver {
    fn run(rom: &[u8], speed: Option<u64>, paused: bool);
}

cfg_if! {
    if #[cfg(any(feature = "egui",target_family = "wasm"))] {
        #[wasm_bindgen]
        pub fn run(rom: &[u8], speed: Option<u64>, paused: bool){
            <frontend::egui::EguiDriver as Chip8Driver>::run(rom, speed, paused);
        }
    } else {
        pub fn run(rom: &[u8], speed: Option<u64>, paused: bool){
            <frontend::raylib::RaylibDriver as Chip8Driver>::run(rom, speed, paused);
        }
    }
}
