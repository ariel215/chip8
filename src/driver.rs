use crate::{
    frontend::{Chip8Frontend, KeyInput},
    Chip8, Chip8Driver, EmulatorMode, RaylibDisplay,
};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};
use wasm_bindgen::prelude::*;

pub const FRAME_DURATION: Duration = Duration::from_millis(1000 / 60);

#[wasm_bindgen]
impl Chip8Driver {
    pub fn new(speed: Option<u64>) -> Self {
        Self {
            chip8: Chip8::init(speed),
            frontend: RaylibDisplay::new(),
            mode: EmulatorMode::Paused,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.chip8.load_rom(rom)
    }

    pub fn pause(&mut self) {
        self.mode = EmulatorMode::Paused;
    }

    pub fn step_paused(&mut self) {
        for k in self.frontend.get_inputs() {
            match k {
                KeyInput::Step => {
                    self.chip8.do_instruction();
                    self.chip8.tick_timers();
                }
                KeyInput::Chip8Key(val) => {
                    self.chip8.clear_keys();
                    self.chip8.set_key(val)
                }
                KeyInput::TogglePause => self.mode = EmulatorMode::Running,
                KeyInput::ToggleDebug => self.frontend.toggle_debug(),
                KeyInput::Click(position) => self.frontend.on_mouse_click(position),
                KeyInput::Scroll(position, amount) => {
                    self.frontend.on_mouse_scroll(position, amount);
                }
            }
        }
    }

    pub fn step_running(&mut self) {
        // At the beginning of each frame, we:
        // - clear the key buffer
        // - tick down the delay and sound registers
        self.chip8.clear_keys();
        self.chip8.tick_timers();

        let cycles_per_frame = 1000 * self.chip8.clock_speed as u32 / 60;
        for _ in 0..cycles_per_frame {
            for k in self.frontend.get_inputs() {
                match k {
                    KeyInput::Chip8Key(key) => self.chip8.set_key(key),
                    KeyInput::Step => {}
                    KeyInput::TogglePause => self.mode = EmulatorMode::Paused,
                    KeyInput::ToggleDebug => self.frontend.toggle_debug(),
                    _ => {}
                }
            }
            if matches!(self.mode, EmulatorMode::Running) {
                self.chip8.do_instruction();
            }
            if self.frontend.is_breakpoint(self.chip8.pc()) {
                self.mode = EmulatorMode::Paused;
            }
        }
    }

    fn draw(&mut self) -> bool {
        self.frontend
            .update(&self.chip8, matches!(self.mode, EmulatorMode::Running))
    }

    pub fn step(&mut self) {
        match self.mode {
            EmulatorMode::Paused => self.step_paused(),
            EmulatorMode::Running => self.step_running(),
        };
    }

    pub fn run(&mut self) {
        loop {
            let start = Instant::now();
            self.step();
            if self.draw() {
                return;
            }
            let elapsed = Instant::now() - start;
            sleep(FRAME_DURATION - elapsed);
        }
    }
}
