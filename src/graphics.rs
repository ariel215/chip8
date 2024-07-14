use std::{collections::HashMap};

use raylib::{self, color::Color, drawing::RaylibDraw, RaylibBuilder, RaylibHandle, RaylibThread, consts::KeyboardKey};

use crate::Display;
pub trait Chip8Frontend{
    fn update(&mut self, display: &Display) -> bool;
    fn get_input(&mut self)->Option<u8>;
}

pub struct RaylibDisplay{
    raylib_handle: RaylibHandle,
    raylib_thread: RaylibThread,
    keymap: HashMap<KeyboardKey,u8>
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
    pub fn new()->Self{
        let (rhandle, rthread) = RaylibBuilder::default()
            .width(Self::WINDOW_WIDTH)
            .height(Self::WINDOW_HEIGHT)
            .resizable()
            .title("Chip-8")
            .build();
        let keymap: HashMap<KeyboardKey, u8> = HashMap::from_iter(
            Self::KEYMAP.iter().copied()
        );
        Self{
            raylib_handle:rhandle,
            raylib_thread:rthread,
            keymap
        }
    }
}

impl Chip8Frontend for RaylibDisplay{
    fn update(&mut self, display: &Display) -> bool {
        let width = self.raylib_handle.get_screen_width() / crate::DISPLAY_COLUMNS as i32;
        let height = self.raylib_handle.get_screen_height() / crate::DISPLAY_ROWS as i32;
        {        
            let mut handle = self.raylib_handle.begin_drawing(&self.raylib_thread);
            handle.clear_background(Color::BLACK);
            for x in 0..crate::DISPLAY_COLUMNS{
                for y in 0..crate::DISPLAY_ROWS{
                    let pixel = display[[x,y]];
                    if pixel {
                        handle.draw_rectangle(x as i32 * width, y as i32 * height, width, height, Color::WHITE)
                    }
                }
            }
        }
        return self.raylib_handle.window_should_close()
    }
    
    fn get_input(&mut self) ->Option<u8>{
        self.raylib_handle.get_key_pressed().map(
            |k| self.keymap.get(&k).copied()
        ).flatten()
    }
}