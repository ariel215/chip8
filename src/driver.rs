use crate::{frontend::{Chip8Frontend, KeyInput}, Chip8, Chip8Driver, EmulatorMode};
use std::{thread::sleep, time::{Duration, Instant}};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch="wasm32", wasm_bindgen)]
impl<Frontend:Chip8Frontend> Chip8Driver<Frontend>{

    pub const FRAME_DURATION: Duration = Duration::from_millis(1000/60);

    pub fn new(speed: Option<u64>) -> Self{
        Self { 
            chip8: Chip8::init(speed),
            frontend: Frontend::new(),
            mode: EmulatorMode::Paused
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]){
        self.chip8.load_rom(rom)
    }

    pub fn pause(&mut self){
        self.mode = EmulatorMode::Paused;
    }


    pub fn step_paused(&mut self) -> bool {
        for k in self.frontend.get_inputs(){
            match k {
                KeyInput::Step => {
                    self.chip8.do_instruction();
                    self.chip8.tick_timers();
                },
                KeyInput::Chip8Key(val) => {
                    self.chip8.clear_keys();
                    self.chip8.set_key(val)
                }
                KeyInput::TogglePause => self.mode = EmulatorMode::Running,
                KeyInput::ToggleDebug => {self.frontend.toggle_debug()},
                KeyInput::Click(position) => {
                    self.frontend.on_mouse_click(position)
                },
                KeyInput::Scroll(position,amount ) => {
                    self.frontend.on_mouse_scroll(position, amount);
                }
            }
        }
        return self.frontend.update(&self.chip8, false)
    }

    pub fn step_running(&mut self) -> bool{
        // At the beginning of each frame, we: 
        // - clear the key buffer
        // - tick down the delay and sound registers
        self.chip8.clear_keys();
        self.chip8.tick_timers();

        let cycles_per_frame = 1000 * self.chip8.clock_speed as u32 / 60;
        for _ in 0..cycles_per_frame{
            for k in self.frontend.get_inputs(){
                match k {
                    KeyInput::Chip8Key(key) => {
                        self.chip8.set_key(key)
                },
                    KeyInput::Step => {},
                    KeyInput::TogglePause => self.mode = EmulatorMode::Paused,
                    KeyInput::ToggleDebug => {self.frontend.toggle_debug()},
                    _ => {}, 
                }
            }
            if matches!(self.mode, EmulatorMode::Running){
                self.chip8.do_instruction();
            }
            if self.frontend.is_breakpoint(self.chip8.pc()){
                self.mode = EmulatorMode::Paused;
            }
        }
        return self.frontend.update(&self.chip8, true);
    }

    pub fn step(&mut self)->bool{
        match  self.mode {
            EmulatorMode::Paused => self.step_paused(),
            EmulatorMode::Running => self.step_running()
        }
    }

    pub fn run(&mut self){
        loop {
            let start = Instant::now();
            self.step();
            let elapsed = Instant::now() - start;
            sleep(Self::FRAME_DURATION - elapsed);
        }
    }
}