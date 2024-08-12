use std::{collections::HashMap, time::{self, Duration}};

use itertools::Itertools;
use raylib::{self, audio::{RaylibAudio, Sound}, color::Color, consts::KeyboardKey, drawing::RaylibDraw, ffi::Vector2, math::Rectangle, RaylibBuilder, RaylibHandle, RaylibThread};


use crate::{emulator::INSTRUCTION_SIZE, Instruction, Memory, Registers};

#[derive(Clone, Copy)]
pub(crate) enum KeyInput{
    Chip8Key(u8),
    Step,
    TogglePause,
    ToggleDebug
} 


pub(crate) trait Chip8Frontend{
    /// Rendering
    fn update(&mut self, memory: &Memory, registers: &Registers) -> bool;
    /// Keyboard input
    fn get_inputs(&mut self)->Vec<KeyInput>;
    /// Toggle debug mode
    fn toggle_debug(&mut self);
    fn start_sound(&mut self);
    fn end_sound(&mut self);
}

pub struct RaylibDisplay{
    raylib_handle: RaylibHandle,
    raylib_thread: RaylibThread,
    raylib_audio: RaylibAudio,
    sound: Sound,
    keymap: HashMap<KeyboardKey,KeyInput>,
    debug_mode: bool,
    keys_down: Vec<(KeyboardKey,KeyState)>
}

macro_rules! vec2 {
    ($obj: expr) => {
        {Vector2{
            x: $obj.x as f32,
            y: $obj.y as f32
        }}
    };
    ($a:expr, $b: expr) => {
        {Vector2{
            x: $a as f32,
            y: $b as f32
        }}
    }
}

enum KeyState {
    Up,
    Pressed,
    HeldSince(time::Instant)
}

impl RaylibDisplay{
    const WINDOW_WIDTH: i32 = 960;
    const WINDOW_HEIGHT: i32 = 480;
    const KEYMAP: [(KeyboardKey,KeyInput); 20] = [
        (KeyboardKey::KEY_ONE, KeyInput::Chip8Key(0x1)),
        (KeyboardKey::KEY_TWO,  KeyInput::Chip8Key(0x2)),
        (KeyboardKey::KEY_THREE,KeyInput::Chip8Key( 0x3)),
        (KeyboardKey::KEY_FOUR, KeyInput::Chip8Key(0xc)),
        (KeyboardKey::KEY_Q, KeyInput::Chip8Key(0x4)),
        (KeyboardKey::KEY_W, KeyInput::Chip8Key(0x5)),
        (KeyboardKey::KEY_E, KeyInput::Chip8Key(0x6)),
        (KeyboardKey::KEY_R, KeyInput::Chip8Key(0xd)),
        (KeyboardKey::KEY_A, KeyInput::Chip8Key(0x7)),
        (KeyboardKey::KEY_S, KeyInput::Chip8Key(0x8)),
        (KeyboardKey::KEY_D, KeyInput::Chip8Key(0x9)),
        (KeyboardKey::KEY_F, KeyInput::Chip8Key(0xe)),
        (KeyboardKey::KEY_Z, KeyInput::Chip8Key(0xa)),
        (KeyboardKey::KEY_X, KeyInput::Chip8Key(0x0)),
        (KeyboardKey::KEY_C, KeyInput::Chip8Key(0xb)),
        (KeyboardKey::KEY_V, KeyInput::Chip8Key(0xf)),
        (KeyboardKey::KEY_SPACE, KeyInput::TogglePause),
        (KeyboardKey::KEY_P, KeyInput::TogglePause),
        (KeyboardKey::KEY_PERIOD, KeyInput::ToggleDebug),
        (KeyboardKey::KEY_ENTER, KeyInput::Step)
    ];
    const DEBUG_MAIN_WINDOW: Rectangle = Rectangle{x:0.0, y:0.0, width: 0.5, height: 0.5};
    const DEBUG_INSTRUCTION_WINDOW: Rectangle = Rectangle{x:0.0, y:0.5, width: 0.5, height: 0.5};
    const DEBUG_MEMORY_WINDOW: Rectangle = Rectangle{x: 0.5, y:0.0, width: 0.5, height: 0.5};
    const DEBUG_REGISTER_WINDOW: Rectangle = Rectangle{x: 0.5, y:0.5, width: 0.5, height: 0.5};
    const SOUND_FILE: &'static str = "resources/buzz.ogg";

    fn draw_instructions(memory: &Memory, registers: &Registers, screen_dims: Vector2, handle: &mut raylib::prelude::RaylibDrawHandle) {
        let window_before = 0;
        let window_after = 10;
        let ram_slice = &memory.ram[registers.pc - (window_before * INSTRUCTION_SIZE)..registers.pc + (window_after * INSTRUCTION_SIZE)];
        let addr_instrs: Vec<(usize, Instruction)> = ram_slice.iter().enumerate().tuples().map(
            |((i1,b1),(_i2,b2)): ((usize,&u8),(usize,&u8))| {
                (registers.pc - (window_before * INSTRUCTION_SIZE) + i1,
                u16::from_be_bytes([*b1, *b2]).into())}
        ).collect();
        let text = addr_instrs.iter().map(|(addr, instr)| {
            match instr{
                Instruction::Nop => "".to_string(),
                _ => format!("0x{:x}\t\t{}", addr, instr)}
            }
        ).join(";\n");
        handle.draw_rectangle_v(times(vec2!(Self::DEBUG_INSTRUCTION_WINDOW), screen_dims),
            times(vec2!(Self::DEBUG_INSTRUCTION_WINDOW.width, Self::DEBUG_INSTRUCTION_WINDOW.height), screen_dims),
             Color::WHITE);
        handle.draw_text(&text,  
            25,
            (screen_dims.y as f32 * Self::DEBUG_INSTRUCTION_WINDOW.y) as i32 + 10 ,
             18, Color::BLACK);
    }

    
    fn draw_memory(memory: &Memory, registers: &Registers, screen_dims: Vector2, handle: &mut raylib::prelude::RaylibDrawHandle) {
        let window_before = 0;
        let window_after = 8 * 4;
        // characters by lines
        let ram_slice = &memory.ram[registers.i - (window_before * INSTRUCTION_SIZE)..registers.i + (window_after * INSTRUCTION_SIZE)];
        let text = ram_slice.iter().tuples().map(|(b0,b1,b2, b3, b4, b5, b6, b7)| {
            format!("{:2x} {:2x} {:2x} {:2x} {:2x} {:2x} {:2x} {:2x}", b0, b1, b2, b3, b4, b5, b6, b7)
            }
        ).join("\n");
        handle.draw_rectangle_v(times(vec2!(Self::DEBUG_MEMORY_WINDOW), screen_dims),
            times(vec2!(Self::DEBUG_MEMORY_WINDOW.width, Self::DEBUG_MEMORY_WINDOW.height), screen_dims),
            Color::LIGHTGRAY);
        handle.draw_text(&text,  
            (screen_dims.x as f32 * Self::DEBUG_MEMORY_WINDOW.x) as i32 + 5,
            (screen_dims.y as f32 * Self::DEBUG_MEMORY_WINDOW.y) as i32 + 10 ,
            18, Color::BLACK);
        
    }
    fn draw_registers(registers: &Registers, screen_dims: Vector2, handle: &mut raylib::prelude::RaylibDrawHandle) {
        let mut register_desc: Vec<_> = registers.vn.iter().enumerate().map(
            |(index, value)| format!("V{:x}: {:x}", index, value)
        ).collect();
        register_desc.push(format!("delay: {}", registers.delay));
        register_desc.push(format!("sound: {}", registers.sound));
        register_desc.push(format!("pc: {:x}", registers.pc));
        register_desc.push(format!("sp: {:x}", registers.sp));
        register_desc.push(format!("memory: {:x}", registers.i));

        handle.draw_rectangle_v(times(vec2!(Self::DEBUG_REGISTER_WINDOW), screen_dims),
            times(vec2!(Self::DEBUG_REGISTER_WINDOW.width, Self::DEBUG_REGISTER_WINDOW.height), screen_dims),
            Color::DARKGRAY);


        // itertools::tuples() drops any elements that don't fit in a tuple, 
        // so we need to make sure that everything lines up
        while register_desc.len() % 4 != 0{
            register_desc.push(String::new());
        }
        
        let text = register_desc.iter().tuples().map(
            |(v1, v2, v3, v4)| format!("{v1}\t{v2}\t{v3}\t{v4}\t")
        ).join("\n");
        handle.draw_text(&text,
        (screen_dims.x as f32 * Self::DEBUG_REGISTER_WINDOW.x) as i32 + 5,
        (screen_dims.y as f32 * Self::DEBUG_REGISTER_WINDOW.y) as i32 + 10 ,
            18, Color::WHITE);
    }

    
}

impl Default for RaylibDisplay{
    fn default() -> Self {
        let (rhandle, rthread) = RaylibBuilder::default()
            .width(Self::WINDOW_WIDTH)
            .height(Self::WINDOW_HEIGHT)
            .resizable()
            .title("Chip-8")
            .build();
        let keymap: HashMap<KeyboardKey, KeyInput> = HashMap::from_iter(
            Self::KEYMAP.iter().copied()
        );
        let keys_down: Vec<(KeyboardKey, KeyState)> = Vec::from_iter(
            Self::KEYMAP.iter().copied().
            map(|(key,_)| {(key,KeyState::Up)})
        );
        let raudio = RaylibAudio::init_audio_device();
        let sound = Sound::load_sound(Self::SOUND_FILE).unwrap();
        Self{
            raylib_handle:rhandle,
            raylib_thread:rthread,
            keymap,
            raylib_audio: raudio,
            sound,
            debug_mode: false,
            keys_down
        }
    }
}
    
fn times(v1: Vector2, v2: Vector2) -> Vector2{
    vec2!(v1.x * v2.x, v1.y * v2.y)
}

impl Chip8Frontend for RaylibDisplay{

    fn update(&mut self, memory: &Memory, registers: &Registers) -> bool {
        let screen_width = self.raylib_handle.get_screen_width();
        let pixel_width: i32 = ((screen_width / crate::DISPLAY_COLUMNS as i32)  as f32 * (
            if self.debug_mode {Self::DEBUG_MAIN_WINDOW.width } else {1.0}
        )) as i32;
        let screen_height = self.raylib_handle.get_screen_height();
        let pixel_height =((screen_height / crate::DISPLAY_ROWS as i32) as f32 * (
            if self.debug_mode {Self::DEBUG_MAIN_WINDOW.height} else {1.0}  
        )) as i32;
        self.keys_down = self.keys_down.iter().map(
            |(key,state)| {
                (*key, match (self.raylib_handle.is_key_down(*key), state){
                    (true, KeyState::Up) => KeyState::Pressed,
                    (true, KeyState::Pressed) => {
                        KeyState::HeldSince(time::Instant::now())
                    } ,
                    (true, KeyState::HeldSince(t)) => KeyState::HeldSince(t.clone()),
                    (false, _ ) => KeyState::Up,     
                })
            }
        ).collect();
        {        
            let mut handle = self.raylib_handle.begin_drawing(&self.raylib_thread);
            handle.clear_background(Color::BLACK);
            for x in 0..crate::DISPLAY_COLUMNS{
                for y in 0..crate::DISPLAY_ROWS{
                    let pixel = memory.display[[x,y]];
                    if pixel {
                        handle.draw_rectangle(x as i32 * pixel_width, y as i32 * pixel_height, pixel_width, pixel_height, Color::WHITE)
                    }
                }
            }
            if self.debug_mode {
                let screen_dims = vec2!(screen_width, screen_height);
                // Draw instructions 
                Self::draw_instructions(memory, registers, screen_dims, &mut handle);
                        
                // Draw memory view
                Self::draw_memory(memory, registers, screen_dims, &mut handle);

                // Draw register view
                Self::draw_registers(registers, screen_dims, &mut handle);
                }
        }
        self.raylib_handle.window_should_close()
    }
    
    fn get_inputs(&mut self) -> Vec<KeyInput> {
        let delay = Duration::from_millis(250);
        let now = time::Instant::now();
        return self.keys_down.iter().filter_map(|(key,state)|{
            match state {
                KeyState::HeldSince(t) => {
                    if now - *t > delay {Some(self.keymap[key])} else {None}},
                KeyState::Up => None,
                KeyState::Pressed => Some(self.keymap[key])
            }
        }).collect_vec()
    }
    
    fn toggle_debug(&mut self) {
        self.debug_mode = !self.debug_mode;
    }
    
    fn start_sound(&mut self) {
        self.raylib_audio.play_sound(&self.sound)
    }
    
    fn end_sound(&mut self) {
        self.raylib_audio.stop_sound(&self.sound)
    }
}


/* 
|----------------------|----------------------|
|                      |    Memory            |  
|  picture             |                      |  
|                      |                      |  
|----------------------|                      |  
|                      |                      |  
|  instructions        | -------------------- |  
|                      |                      |  
|                      |  registers           |  
-----------------------|----------------------|

*/