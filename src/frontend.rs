use std::{cmp::max, collections::HashMap, time::{self, Duration}};

use bitvec::{array::BitArray, bitarr, BitArr};
use itertools::Itertools;
use raylib::{self, audio::{RaylibAudio, Sound}, color::Color, consts::KeyboardKey, drawing::RaylibDraw, ffi::Vector2, math::Rectangle, text::Font, RaylibBuilder, RaylibHandle, RaylibThread};


use crate::{emulator::INSTRUCTION_SIZE, Chip8, Instruction, MEMORY_SIZE};
#[derive(Clone, Copy)]
pub enum KeyInput{
    Chip8Key(u8),
    Step,
    TogglePause,
    ToggleDebug,
    Click(Vector2),
    Scroll(Vector2, isize)
} 


pub trait Chip8Frontend{
    /// Rendering
    fn update(&mut self, chip8: &crate::Chip8, show_current_instruction: bool) -> bool;
    /// Keyboard input
    fn get_inputs(&mut self)->Vec<KeyInput>;
    /// Toggle debug mode
    fn toggle_debug(&mut self);

    fn on_mouse_scroll(&mut self, position: Vector2, direction: isize);

    fn on_mouse_click(&mut self, position: Vector2);
}

pub struct RaylibDisplay{
    raylib_handle: RaylibHandle,
    raylib_thread: RaylibThread,
    debug_mode: bool,
    font: Font,
    keymap: HashMap<KeyboardKey,KeyInput>,
    keys_down: Vec<(KeyboardKey,KeyState)>,
    instruction_window: InstructionWindow,
    breakpoints: BitArr!(for MEMORY_SIZE)
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


    
    fn draw_memory(chip8: &Chip8, screen_dims: Vector2, handle: &mut raylib::prelude::RaylibDrawHandle) {
        let window_before = 0;
        let window_after = 8 * 4;
        // characters by lines
        let ram_slice = &chip8.memory.ram[chip8.registers.i - (window_before * INSTRUCTION_SIZE)..chip8.registers.i + (window_after * INSTRUCTION_SIZE)];
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
    fn draw_registers(chip8: &Chip8, screen_dims: Vector2, handle: &mut raylib::prelude::RaylibDrawHandle) {
        let registers = &chip8.registers;
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

    pub fn new() -> Self {
        let (mut rhandle, rthread) = RaylibBuilder::default()
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
        rhandle.set_text_line_spacing(InstructionWindow::LINE_SPACING);
        let instruction_window = InstructionWindow{
            start_addr: InstructionWindow::BASE_ADDR,
            len: 8,
            position: Rectangle { 
                x: 0.0,  
                y: Self::WINDOW_HEIGHT as f32 * Self::DEBUG_INSTRUCTION_WINDOW.y,
                width: Self::WINDOW_WIDTH as f32 * Self::DEBUG_INSTRUCTION_WINDOW.width,
                height: Self::WINDOW_HEIGHT as f32 * Self::DEBUG_INSTRUCTION_WINDOW.height 
            }
        };
        let font = rhandle.load_font(&rthread, "resources/fonts/VT323/VT323-Regular.ttf").unwrap();
        Self{
            raylib_handle:rhandle,
            raylib_thread:rthread,
            font,
            keymap,
            debug_mode: false,
            keys_down,
            instruction_window,
            breakpoints: BitArray::ZERO
        }
    }
}
    
fn times(v1: Vector2, v2: Vector2) -> Vector2{
    vec2!(v1.x * v2.x, v1.y * v2.y)
}

impl Chip8Frontend for RaylibDisplay{

    fn update(&mut self, chip8: &Chip8, show_current_instruction: bool) -> bool {
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
                    let pixel = chip8.memory.display[[x,y]];
                    if pixel {
                        handle.draw_rectangle(x as i32 * pixel_width, y as i32 * pixel_height, pixel_width, pixel_height, Color::WHITE)
                    }
                }
            }
            if self.debug_mode {
                let screen_dims = vec2!(screen_width, screen_height);
                // Draw instructions
                self.instruction_window.refresh_position(screen_dims);
                if show_current_instruction{
                    self.instruction_window.start_addr = max(chip8.pc()-(3 * INSTRUCTION_SIZE), InstructionWindow::BASE_ADDR);
                }
                self.instruction_window.draw(&self.font, &self.breakpoints, chip8, &mut handle);
                // Draw memory view
                Self::draw_memory(chip8, screen_dims, &mut handle);

                // Draw register view
                Self::draw_registers(chip8, screen_dims, &mut handle);
                }
        }
        self.raylib_handle.window_should_close()
    }
    
    fn get_inputs(&mut self) -> Vec<KeyInput> {
        let delay = Duration::from_millis(250);
        let now = time::Instant::now();
        let mut inputs = self.keys_down.iter().filter_map(|(key,state)|{
            match state {
                KeyState::HeldSince(t) => {
                    if now - *t > delay {Some(self.keymap[key])} else {None}},
                KeyState::Up => None,
                KeyState::Pressed => Some(self.keymap[key])
            }
        }).collect_vec();
        if self.raylib_handle.is_mouse_button_pressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT) {
            inputs.push(KeyInput::Click(vec2!(self.raylib_handle.get_mouse_x(), self.raylib_handle.get_mouse_y())));
        }
        let mouse_wheel = self.raylib_handle.get_mouse_wheel_move().round() as isize;
        if  mouse_wheel != 0 {
            // negative is down, but everywhere else negative is up, so we invert the scroll amount to match
            inputs.push(KeyInput::Scroll(vec2!(self.raylib_handle.get_mouse_x(), self.raylib_handle.get_mouse_y()), -mouse_wheel))
        }
        inputs
    }
    
    fn toggle_debug(&mut self) {
        self.debug_mode = !self.debug_mode;
    }

    fn on_mouse_click(&mut self, position: Vector2) {
        let screen_dims = vec2!(self.raylib_handle.get_screen_width(), self.raylib_handle.get_screen_height());
        match (((position.x / screen_dims.x) < 0.5), ((position.y / screen_dims.y ) < 0.5)) {
            (true, true) => { // chip8 window
                },
            (true, false) => {
                if let Some(addr)  = self.instruction_window.get_addr(position.y){
                    let prev = *self.breakpoints.get(addr).unwrap();
                    self.breakpoints.set(addr, !prev);
                    dbg!(format!("{:x}",addr));
                }
            } //  instruction view
            (false, true) => {}, //  memory view
            (false, false) => {}, // register view
        }
    }

    fn on_mouse_scroll(&mut self, position: Vector2, direction: isize) {
        let screen_dims = vec2!(self.raylib_handle.get_screen_width(), self.raylib_handle.get_screen_height());
        match (((position.x / screen_dims.x) < 0.5), ((position.y / screen_dims.y ) < 0.5)) {
            (true, true) => { // chip8 window
                },
            (true, false) => {
                self.instruction_window.scroll(direction * (INSTRUCTION_SIZE as isize));
            } //  instruction view
            (false, true) => {}, //  memory view
            (false, false) => {}, // register view
        }
        
    }
    
}


struct InstructionWindow{
    start_addr: usize,
    len: usize,
    position: Rectangle
}



impl InstructionWindow{
    const BASE_ADDR: usize = 0x200;
    const LINE_SPACING: i32 = 20;

    pub(crate) fn draw<T: RaylibDraw>(&self, font: &Font, breakpoints: &BitArr!(for MEMORY_SIZE), chip8: &Chip8, handle: &mut T) {
        let ram_slice = &chip8.memory.ram[self.start_addr..self.start_addr + (self.len * INSTRUCTION_SIZE)];
        let addr_instrs: Vec<(usize, Instruction)> = ram_slice.iter().enumerate().tuples().map(
            |((i1,b1),(_i2,b2)): ((usize,&u8),(usize,&u8))| {
                (self.start_addr + i1,
                u16::from_be_bytes([*b1, *b2]).into())
            }
        ).collect();
        let text = addr_instrs.iter().map(|(addr, instr)| {
                if *addr == chip8.pc() {format!("\t>>0x{:x}\t\t{}", addr, instr)} else{ format!("0x{:x}\t\t{}", addr, instr)}
            }
        ).join(";\n");

        handle.draw_rectangle_v(vec2!(self.position.x, self.position.y),
            vec2!(self.position.width, self.position.height),
             Color::WHITE);
        handle.draw_text_ex(font,
            &text,  
            vec2!(25,self.position.y + 10.0),
             32.0, 1.0, Color::BLACK);
    }

    pub(crate) fn scroll(&mut self, direction: isize){
        match self.start_addr.checked_add_signed(direction){
            Some(addr) => self.start_addr = addr,
            None => self.start_addr = 0
        }
    }

    pub(crate) fn get_addr(&self, y: f32) -> Option<usize>{
        let offset = y - self.position.y;
        if offset.is_sign_negative() {
            None
        } else {
            let line_no = (offset / self.len as f32).trunc() as usize;
            Some((line_no * INSTRUCTION_SIZE) + self.start_addr)
        }
    }
    
    fn refresh_position(&mut self, screen_dims: Vector2) {
        let xy = times(screen_dims, vec2!(RaylibDisplay::DEBUG_INSTRUCTION_WINDOW));
        let wh = times(screen_dims, vec2!(RaylibDisplay::DEBUG_INSTRUCTION_WINDOW.width,RaylibDisplay::DEBUG_INSTRUCTION_WINDOW.height));
        self.position = Rectangle{
            x: xy.x,
            y: xy.y,
            width: wh.x,
            height: wh.y
        };
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