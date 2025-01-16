use cfg_if::cfg_if;

use crate::frontend::egui::EguiDisplay;
use crate::frontend::raylib::RaylibDisplay;
use crate::frontend::Chip8Frontend;
use crate::{
    frontend::KeyInput,
    Chip8, Chip8Driver, EmulatorMode,
};
use std::process::exit;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

pub const FRAME_DURATION: Duration = Duration::from_millis(1000 / 60);

impl Chip8Driver {
    pub fn new(speed: Option<u64>) -> Self {
        cfg_if!{
            if #[cfg(feature = "egui")] {
                Self {
                    chip8: Chip8::init(speed),
                    frontend: Box::new(EguiDisplay::default()),
                    mode: EmulatorMode::Running
                }
            } else {
                Self {
                    chip8: Chip8::init(speed),
                    frontend: Box::new(RaylibDisplay::default()),
                    mode: EmulatorMode::Paused
                }
            }
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

        let cycles_per_frame = self.chip8.clock_speed / 60;
        for _ in 0..cycles_per_frame {
            for k in self.frontend.get_inputs() {
                match k {
                    KeyInput::Chip8Key(key) => self.chip8.set_key(key),
                    KeyInput::Step => {},
                    KeyInput::TogglePause => {
                        self.mode = EmulatorMode::Paused;
                        break;},
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

    #[cfg(feature = "egui")]
    pub fn run(speed: Option<u64>, instructions: Vec<u8>){
        let conf = miniquad::conf::Conf::default();
        miniquad::start(conf, move ||{
            let mut driver = Self::new(speed);
            driver.load_rom(&instructions);
            Box::new(driver)
        })
    }

    #[cfg(not(feature = "egui"))]
    pub fn run(&mut self) {
        loop {
            let start = Instant::now();
            self.step();
            if self.draw() {
                return;
            }
            let elapsed = Instant::now().duration_since(start);
            sleep(FRAME_DURATION - elapsed);
        }
    }
}


#[cfg(feature = "egui")]
impl miniquad::EventHandler for Chip8Driver {
    fn update(&mut self) {
        self.step();
    }

    fn draw(&mut self) {
        Chip8Driver::draw(self);
    }
    
    
    fn window_minimized_event(&mut self) {
        self.pause();
    }
    
    fn window_restored_event(&mut self) {
        self.mode = EmulatorMode::Running;
    }
    
    fn quit_requested_event(&mut self) {
        exit(0);
    }
}
