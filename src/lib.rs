use cfg_if::cfg_if;
use frontend::Chip8Frontend;
use ndarray::prelude::*;

use wasm_bindgen::prelude::*;

type Display = Array2<bool>;

pub mod driver;
pub mod emulator;
pub mod errors;
pub mod frontend;
pub mod instructions;

pub use emulator::Chip8;
pub(crate) use emulator::{DISPLAY_COLUMNS, DISPLAY_ROWS};
pub use instructions::Instruction;
