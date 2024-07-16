use std::{cmp::min, collections::HashMap};

use itertools::Itertools;
use raylib::{self, color::Color, consts::KeyboardKey, drawing::RaylibDraw, ffi::Vector2, math::Rectangle, RaylibBuilder, RaylibHandle, RaylibThread};

use crate::{emulator::{self, INSTRUCTION_SIZE}, Display, Instruction, Memory, Registers};

#[derive(Clone, Copy)]
pub(crate) enum KeyInput{
    Chip8Key(u8),
    DebugStep
} 


pub(crate) trait Chip8Frontend{
    /// Rendering
    fn update(&mut self, memory: &Memory, registers: &Registers) -> bool;
    /// Keyboard input
    fn get_input(&mut self)->Option<KeyInput>;
    /// Toggle debug mode
    fn debug_mode(&mut self);
}

pub struct RaylibDisplay{
    raylib_handle: RaylibHandle,
    raylib_thread: RaylibThread,
    keymap: HashMap<KeyboardKey,KeyInput>,
    debug_mode: bool
}

impl RaylibDisplay{
    const WINDOW_WIDTH: i32 = 960;
    const WINDOW_HEIGHT: i32 = 480;
    const KEYMAP: [(KeyboardKey,u8); 16] = [
        (KeyboardKey::KEY_ONE, 0x1),
        (KeyboardKey::KEY_TWO, 0x2),
        (KeyboardKey::KEY_THREE, 0x3),
        (KeyboardKey::KEY_FOUR, 0xc),
        (KeyboardKey::KEY_Q, 0x4),
        (KeyboardKey::KEY_W, 0x5),
        (KeyboardKey::KEY_E, 0x6),
        (KeyboardKey::KEY_R, 0xd),
        (KeyboardKey::KEY_A, 0x7),
        (KeyboardKey::KEY_S, 0x8),
        (KeyboardKey::KEY_D, 0x9),
        (KeyboardKey::KEY_F, 0xe),
        (KeyboardKey::KEY_Z, 0xa),
        (KeyboardKey::KEY_X, 0x0),
        (KeyboardKey::KEY_C, 0xb),
        (KeyboardKey::KEY_V, 0xf)
    ];
    const DEBUG_MAIN_WINDOW: Rectangle = Rectangle{x:0.0, y:0.0, width: 0.5, height: 0.5};
    const DEBUG_INSTRUCTION_WINDOW: Rectangle = Rectangle{x:0.0, y:0.5, width: 0.5, height: 0.5};
    const DEBUG_MEMORY_WINDOW: Rectangle = Rectangle{x: 0.5, y:0.0, width: 0.5, height: 0.5};
    const DEBUG_REGISTER_WINDOW: Rectangle = Rectangle{x: 0.5, y:0.5, width: 0.5, height: 0.5};
    pub fn new()->Self{
        let (rhandle, rthread) = RaylibBuilder::default()
            .width(Self::WINDOW_WIDTH)
            .height(Self::WINDOW_HEIGHT)
            .resizable()
            .title("Chip-8")
            .build();
        let mut keymap: HashMap<KeyboardKey, KeyInput> = HashMap::from_iter(
            Self::KEYMAP.iter().copied().map(|(k,v)|(k,KeyInput::Chip8Key(v)))
        );
        keymap.insert(KeyboardKey::KEY_ENTER,KeyInput::DebugStep);
        Self{
            raylib_handle:rhandle,
            raylib_thread:rthread,
            keymap,
            debug_mode: false
        }
    }
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
                // Draw instructions 
                let window_before = 0;
                let window_after = 10;
                let ram_slice = &memory.ram[registers.pc - (window_before * INSTRUCTION_SIZE)..registers.pc + (window_after * INSTRUCTION_SIZE)];
                let addr_instrs: Vec<(usize, Instruction)> = ram_slice.iter().tuples().enumerate().map(
                    |(i,bytes): (usize,(&u8,&u8))| {
                        (registers.pc - (window_before * INSTRUCTION_SIZE) + i,
                        u16::from_be_bytes([*bytes.0, *bytes.1]).into())}
                ).collect();
                let text = addr_instrs.iter().map(|(addr, instr)| {
                    match instr{
                        Instruction::Nop => "".to_string(),
                        _ => format!("0x{:x}\t\t{}", addr, instr)}
                    }
                ).join(";\n");
                let screen_dims = vec2!(screen_width, screen_height);
                handle.draw_rectangle_v(times(vec2!(Self::DEBUG_INSTRUCTION_WINDOW), screen_dims),
                    times(vec2!(Self::DEBUG_INSTRUCTION_WINDOW.width, Self::DEBUG_INSTRUCTION_WINDOW.height), screen_dims),
                     Color::WHITE);
                handle.draw_text(&text,  
                    25,
                    (screen_height as f32 * Self::DEBUG_INSTRUCTION_WINDOW.y) as i32 + 10 ,
                     18, Color::BLACK);
        
                // Draw memory view
                let window_before = 0;
                let window_after = 8 * 4; // characters by lines
                let ram_slice = &memory.ram[registers.i - (window_before * INSTRUCTION_SIZE)..registers.i + (window_after * INSTRUCTION_SIZE)];
                let text = ram_slice.iter().tuples().map(|(b0,b1,b2, b3, b4, b5, b6, b7)| {
                    format!("{:2x} {:2x} {:2x} {:2x} {:2x} {:2x} {:2x} {:2x}", b0, b1, b2, b3, b4, b5, b6, b7)
                    }
                ).join("\n");
                let screen_dims = vec2!(screen_width, screen_height);
                handle.draw_rectangle_v(times(vec2!(Self::DEBUG_MEMORY_WINDOW), screen_dims),
                    times(vec2!(Self::DEBUG_MEMORY_WINDOW.width, Self::DEBUG_MEMORY_WINDOW.height), screen_dims),
                     Color::LIGHTGRAY);
                handle.draw_text(&text,  
                    (screen_width as f32 * Self::DEBUG_MEMORY_WINDOW.x) as i32 + 5,
                    (screen_height as f32 * Self::DEBUG_MEMORY_WINDOW.y) as i32 + 10 ,
                     18, Color::BLACK);

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
                (screen_width as f32 * Self::DEBUG_REGISTER_WINDOW.x) as i32 + 5,
                (screen_height as f32 * Self::DEBUG_REGISTER_WINDOW.y) as i32 + 10 ,
                    18, Color::WHITE);
                }
        }

        return self.raylib_handle.window_should_close()
    }
    
    fn get_input(&mut self) -> Option<KeyInput> {
        self.raylib_handle.get_key_pressed().map(
            |k| self.keymap.get(&k).copied()
        ).flatten()
    }
    
    fn debug_mode(&mut self) {
        self.debug_mode = true;
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