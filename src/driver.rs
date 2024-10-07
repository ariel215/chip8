use crate::{frontend::{KeyInput, RaylibDisplay}, Chip8, Chip8Driver, EmulatorMode};
use raylib::audio::RaylibAudio;
use std::{thread::sleep, time::{Duration, Instant}};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch="wasm32", wasm_bindgen)]
impl Chip8Driver{

    #[cfg(target_arch="wasm32")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(construtor))]
    pub fn new(mode: EmulatorMode, speed: Option<u64>) -> Self{

        let mut driver = Self { 
            chip8: Chip8::init(speed),
            frontend: Box::new(WebDisplay::new()),
            mode
        };
        if matches!(mode, EmulatorMode::Paused){
            driver.frontend.toggle_debug();
        }
        driver
    }

    #[cfg(not(target_arch="wasm32"))]
    pub fn new(mode: EmulatorMode, speed: Option<u64>) -> Self{
        
        let mut driver = Self { 
            chip8: Chip8::init(speed),
            frontend: Box::new(RaylibDisplay::new()),
            mode
        };
        if matches!(mode, EmulatorMode::Paused){
            driver.frontend.toggle_debug();
        }
        driver
    }

    pub fn load_rom(&mut self, rom: &[u8]){
        self.chip8.load_rom(rom)
    }

    pub fn pause(&mut self){
        self.mode = EmulatorMode::Paused;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(&mut self){

        let audio = RaylibAudio::init_audio_device().unwrap();
        let sound = audio.new_wave_from_memory("ogg",RaylibDisplay::SOUND_FILE).unwrap();
        let mut sound = audio.new_sound_from_wave(&sound).unwrap();
    
        let cycle_length = Duration::from_millis(1000 / self.chip8.clock_speed);
        let frame_length = Duration::from_millis(1000/60);
        loop {
            match self.mode{
                EmulatorMode::Paused => {
                for k in self.frontend.get_inputs(){
                    match k {
                        KeyInput::Step => {
                            self.chip8.do_instruction();
                            self.chip8.tick_timers();
                            if self.frontend.update(&self.chip8, true) {return}

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
                if self.frontend.update(&self.chip8, false) {return;}
                sleep(Duration::from_millis(50));
            },
            EmulatorMode::Running => {
                let mut frame_elapsed = Duration::ZERO;
                // At the beginning of each frame, we: 
                // - clear the key buffer
                // - tick down the delay and sound registers
                self.chip8.clear_keys();
                self.chip8.tick_timers();

                while frame_elapsed < frame_length{
                    let tic = Instant::now();
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
                    let toc = Instant::now();
                    if toc - tic < cycle_length{
                        sleep(cycle_length - (toc-tic))
                    }
                    frame_elapsed += Instant::now() - tic;
                }
                // At the end of each frame, update the screen and toggle 
                if sound.is_playing() & !self.chip8.sound(){
                    sound.stop();
                }
                if !sound.is_playing() & self.chip8.sound(){
                    sound.play()
                }
                if self.frontend.update(&self.chip8, true){
                    break;
                }
            }
        }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn step_paused(&mut self){
        for k in self.frontend.get_inputs(){
            match k {
                KeyInput::Step => {
                    self.chip8.do_instruction();
                    self.chip8.tick_timers();
                    if self.frontend.update(&self.chip8, true) {return}

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
    }

    #[cfg(target_arch = "wasm32")]
    pub fn run(&mut self){

        let audio = RaylibAudio::init_audio_device().unwrap();
        let sound = audio.new_wave_from_memory("ogg",RaylibDisplay::SOUND_FILE).unwrap();
        let mut sound = audio.new_sound_from_wave(&sound).unwrap();
    
        let cycle_length = Duration::from_millis(1000 / self.chip8.clock_speed);
        let frame_length = Duration::from_millis(1000/60);
        loop {
            match self.mode{
                EmulatorMode::Paused => {
                for k in self.frontend.get_inputs(){
                    match k {
                        KeyInput::Step => {
                            self.chip8.do_instruction();
                            self.chip8.tick_timers();
                            if self.frontend.update(&self.chip8, true) {return}

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
                if self.frontend.update(&self.chip8, false) {return;}
                sleep(Duration::from_millis(50));
            },
            EmulatorMode::Running => {
                let mut frame_elapsed = Duration::ZERO;
                // At the beginning of each frame, we: 
                // - clear the key buffer
                // - tick down the delay and sound registers
                self.chip8.clear_keys();
                self.chip8.tick_timers();

                while frame_elapsed < frame_length{
                    let mut cycle_elapsed = 0;
                    let tic = Instant::now();
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
                    frame_elapsed += Instant::now() - tic;
                }
                // At the end of each frame, update the screen and toggle 
                if sound.is_playing() & !self.chip8.sound(){
                    sound.stop();
                }
                if !sound.is_playing() & self.chip8.sound(){
                    sound.play()
                }
                if self.frontend.update(&self.chip8, true){
                    break;
                }
            }
        }
        }
    }

}

