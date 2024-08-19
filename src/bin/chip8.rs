use std::{io::Read, thread::sleep, time::{Duration, Instant}};

use chip8::{frontend::KeyInput, Chip8};

use clap::Parser;
use clio::*;
use raylib::audio::RaylibAudio;

#[derive(Parser)]
struct Args{
    rom: ClioPath,
    #[arg(short, long)]
    speed: Option<u64>,
    #[arg(short, long)]
    debug: bool
}

#[derive(Clone, Copy)]
enum EmulatorMode {
    Running,
    Paused, 
}

struct Chip8Driver{
    chip8: chip8::Chip8,   
    frontend: Box<dyn chip8::frontend::Chip8Frontend>,
    mode: EmulatorMode
}


impl Chip8Driver{

    pub fn new(mode: EmulatorMode) -> Self{

        let mut driver = Self { 
            chip8: Chip8::init(), 
            frontend: Box::new(chip8::frontend::RaylibDisplay::new()),
            mode
        };
        if matches!(mode, EmulatorMode::Paused){
            driver.frontend.toggle_debug();
        }
        driver
    }

    pub fn run(&mut self){

        let audio = RaylibAudio::init_audio_device().unwrap();
        let mut sound = audio.new_sound("resources/buzz.ogg").unwrap();
    
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
                        },
                        KeyInput::Chip8Key(val) => {
                            self.chip8.clear_keys();
                            self.chip8.set_key(val)
                        }
                        KeyInput::TogglePause => self.mode = EmulatorMode::Running,
                        KeyInput::ToggleDebug => {self.frontend.toggle_debug()}
                    }
                }
                if self.frontend.update(&self.chip8) {break;}
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
                            KeyInput::ToggleDebug => {self.frontend.toggle_debug()}
                        }
                    }
                    self.chip8.do_instruction();
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
                if self.frontend.update(&self.chip8){
                    break;
                }
            }
        }
        }
    }

}


fn main() {
    let args = Args::parse();
    let rom_name = args.rom.as_os_str().to_string_lossy().into_owned();
    let mut input = args.rom.open().expect(&format!("No file named {}", rom_name));
    let mut instructions = Vec::new();
    input.read_to_end(&mut instructions).expect(&format!("Failed to read {}", rom_name ));
    let mut driver = Chip8Driver::new(if args.debug {EmulatorMode::Paused} else {EmulatorMode::Running});
    if args.speed.is_some(){
        driver.chip8 = driver.chip8.clock_speed(args.speed.unwrap());
    }
    driver.chip8 = driver.chip8.load_rom(&instructions);
    driver.run()
}
