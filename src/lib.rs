use cfg_if::cfg_if;
use frontend::Chip8Frontend;
use ndarray::prelude::*;

use wasm_bindgen::prelude::*;

type Display = Array2<bool>;

pub mod emulator;
pub mod driver;
pub mod errors;
pub mod frontend;
pub mod instructions;

pub use emulator::Chip8;
pub use instructions::Instruction;
pub(crate) use emulator::{DISPLAY_COLUMNS, DISPLAY_ROWS};


