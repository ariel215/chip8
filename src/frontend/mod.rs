pub mod egui;
pub mod raylib;

use itertools::Itertools;
use std::cmp::max;
use std::default;
use std::ops::Range;

use crate::emulator::INSTRUCTION_SIZE;
use crate::{Chip8, Frontend, Instruction};

#[derive(Clone, Copy)]
pub enum KeyInput {
    Chip8Key(u8),
    Step,
    TogglePause,
    ToggleDebug,
    Click(Vector),
    Scroll(Vector, isize),
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Vector {
    x: f32,
    y: f32,
}

pub trait Chip8Frontend {
    /// Rendering and sound
    fn update(&mut self, chip8: &crate::Chip8, show_current_instruction: bool) -> bool;
    /// Keyboard input
    fn get_inputs(&mut self) -> Vec<KeyInput>;
    /// Toggle debug mode
    fn toggle_debug(&mut self);
    fn is_breakpoint(&self, addr: usize) -> bool;

    fn on_mouse_scroll(&mut self, position: Vector, direction: isize);

    fn on_mouse_click(&mut self, position: Vector);

    fn kind(&self) -> Frontend;

}

fn print_memory(chip8: &Chip8) -> String {
    let window_before = 0;
    let window_after = 8 * 4;
    // characters by lines
    let ram_slice = &chip8.memory.ram[chip8.registers.i - (window_before * INSTRUCTION_SIZE)
        ..chip8.registers.i + (window_after * INSTRUCTION_SIZE)];
    ram_slice
        .iter()
        .tuples()
        .map(|(b0, b1, b2, b3, b4, b5, b6, b7)| {
            format!(
                "{:2x} {:2x} {:2x} {:2x} {:2x} {:2x} {:2x} {:2x}",
                b0, b1, b2, b3, b4, b5, b6, b7
            )
        })
        .join("\n")
}

fn print_registers(chip8: &Chip8) -> String {
    let registers = &chip8.registers;
    let mut register_desc: Vec<_> = registers
        .vn
        .iter()
        .enumerate()
        .map(|(index, value)| format!("V{:x}: {:x}", index, value))
        .collect();
    register_desc.push(format!("delay: {}", registers.delay));
    register_desc.push(format!("sound: {}", registers.sound));
    register_desc.push(format!("pc: {:x}", registers.pc));
    register_desc.push(format!("sp: {:x}", registers.sp));
    register_desc.push(format!("memory: {:x}", registers.i));

    // itertools::tuples() drops any elements that don't fit in a tuple,
    // so we need to make sure that everything lines up
    while register_desc.len() % 4 != 0 {
        register_desc.push(String::new());
    }

    register_desc
        .iter()
        .tuples()
        .map(|(v1, v2, v3, v4)| format!("{v1}\t{v2}\t{v3}\t{v4}\t"))
        .join("\n")
}


struct InstructionWindow {
    start_addr: usize,
    len: usize,
}

impl InstructionWindow {
    const BASE_ADDR: usize = 0x200;

    pub(crate) fn scroll(&mut self, direction: isize) {
        self.start_addr = self.start_addr.wrapping_add_signed(direction)
    }

    pub fn focus(&mut self, addr: usize) {
        self.start_addr = max(addr - (3 * INSTRUCTION_SIZE), InstructionWindow::BASE_ADDR);
    }

    pub(crate) fn range(&self) -> Range<usize> {
        self.start_addr..(self.start_addr + self.len * INSTRUCTION_SIZE)
    }

    pub fn lines(&self, chip8: &Chip8) -> Vec<(usize, String)> {
        let start_addr = self.start_addr;
        let ram_slice = &chip8.memory.ram[self.range()];
        ram_slice
            .iter()
            .enumerate()
            .tuples()
            .map(|((i1, b1), (_i2, b2)): ((usize, &u8), (usize, &u8))| {
                (start_addr + i1, u16::from_be_bytes([*b1, *b2]).into())
            })
            .map(|(addr, instr): (usize, Instruction)| {
                (
                    addr,
                    if addr == chip8.pc() {
                        format!("\t>>0x{:x}\t\t{}", addr, instr)
                    } else {
                        format!("0x{:x}\t\t{}", addr, instr)
                    },
                )
            })
            .collect_vec()
    }
}

impl Default for InstructionWindow {
    fn default() -> Self {
        Self {
            start_addr: Self::BASE_ADDR,
            len: 8,
        }
    }
}
