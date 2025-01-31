use std::{
    collections::HashMap,
    sync::LazyLock,
    thread::sleep,
    time::{self, Duration, Instant},
};

use super::{print_memory, print_registers, KeyInput, FRAME_DURATION};
use crate::{
    driver::{Chip8Driver, EmulatorMode},
    emulator::{INSTRUCTION_SIZE, MEMORY_SIZE},
    Chip8,
};
use ::raylib::{
    self,
    audio::{RaylibAudio, Sound},
    color::Color,
    consts::KeyboardKey,
    drawing::RaylibDraw,
    ffi::Vector2,
    math::Rectangle,
    text::Font,
    RaylibBuilder, RaylibHandle, RaylibThread,
    logging
};

use bitvec::{array::BitArray, BitArr};
use itertools::Itertools;

impl From<Vector2> for super::Vector {
    fn from(value: Vector2) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

pub(crate) struct RaylibDriver {
    pub(crate) display: RaylibDisplay,
    pub(crate) chip8: Chip8,
    pub(crate) mode: EmulatorMode,
}

impl Chip8Driver for RaylibDriver {
    fn run(rom: &[u8], speed: Option<u64>, paused: bool) {
        let mut driver = Self::new(speed, paused);
        driver.load_rom(rom);
        loop {
            let start = Instant::now();
            driver.step();
            if driver.draw() {
                return;
            }
            let elapsed = Instant::now().duration_since(start);
            sleep(FRAME_DURATION - elapsed);
        }
    }
}

impl RaylibDriver {
    pub fn new(speed: Option<u64>, paused: bool) -> Self {
        Self {
            chip8: Chip8::init(speed),
            display: RaylibDisplay::default(),
            mode: if paused {
                EmulatorMode::Paused
            } else {
                EmulatorMode::Running
            },
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.chip8.load_rom(rom);

    }

    pub fn step_paused(&mut self) {
        for k in self.display.get_inputs() {
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
                KeyInput::ToggleDebug => self.display.toggle_debug(),
                KeyInput::Click(position) => self.display.on_mouse_click(position),
                KeyInput::Scroll(position, amount) => {
                    self.display.on_mouse_scroll(position, amount);
                }
                other => print!("unknown key input: {:?}", other)
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
            for k in self.display.get_inputs() {
                match k {
                    KeyInput::Chip8Key(key) => self.chip8.set_key(key),
                    KeyInput::Step => {}
                    KeyInput::TogglePause => {
                        self.mode = EmulatorMode::Paused;
                        break;
                    }
                    KeyInput::ToggleDebug => self.display.toggle_debug(),
                    _ => {}
                }
            }
            if matches!(self.mode, EmulatorMode::Running) {
                self.chip8.do_instruction();
            }
            if self.display.is_breakpoint(self.chip8.pc()) {
                self.mode = EmulatorMode::Paused;
            }
        }
    }

    fn draw(&mut self) -> bool {
        self.display
            .update(&self.chip8, matches!(self.mode, EmulatorMode::Running))
    }

    pub fn step(&mut self) {
        match self.mode {
            EmulatorMode::Paused => self.step_paused(),
            EmulatorMode::Running => self.step_running(),
        };
    }
}

pub(crate) struct RaylibDisplay {
    raylib_handle: RaylibHandle,
    raylib_thread: RaylibThread,
    raylib_sound: Option<Sound<'static>>,
    debug_mode: bool,
    font: Option<Font>,
    keymap: HashMap<KeyboardKey, KeyInput>,
    keys_down: Vec<(KeyboardKey, KeyState)>,
    instruction_window: RaylibInstructionWindow,
    breakpoints: BitArr!(for MEMORY_SIZE),
}

macro_rules! vec2 {
    ($obj: expr) => {{
        Vector2 {
            x: $obj.x as f32,
            y: $obj.y as f32,
        }
    }};
    ($a:expr, $b: expr) => {{
        Vector2 {
            x: $a as f32,
            y: $b as f32,
        }
    }};
}

#[derive(Clone, Copy)]
enum KeyState {
    Up,
    Pressed,
    HeldSince(time::Instant),
}

impl RaylibDisplay {
    const WINDOW_WIDTH: i32 = 960;
    const WINDOW_HEIGHT: i32 = 480;
    const KEYMAP: [(KeyboardKey, KeyInput); 20] = [
        (KeyboardKey::KEY_ONE, KeyInput::Chip8Key(0x1)),
        (KeyboardKey::KEY_TWO, KeyInput::Chip8Key(0x2)),
        (KeyboardKey::KEY_THREE, KeyInput::Chip8Key(0x3)),
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
        (KeyboardKey::KEY_ENTER, KeyInput::Step),
    ];
    const DEBUG_MAIN_WINDOW: Rectangle = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 0.5,
        height: 0.5,
    };
    const DEBUG_INSTRUCTION_WINDOW: Rectangle = Rectangle {
        x: 0.0,
        y: 0.5,
        width: 0.5,
        height: 0.5,
    };
    const DEBUG_MEMORY_WINDOW: Rectangle = Rectangle {
        x: 0.5,
        y: 0.0,
        width: 0.5,
        height: 0.5,
    };
    const DEBUG_REGISTER_WINDOW: Rectangle = Rectangle {
        x: 0.5,
        y: 0.5,
        width: 0.5,
        height: 0.5,
    };

    pub const SOUND_FILE: &'static [u8] = include_bytes!("../../resources/buzz.ogg");
    pub const FONT_FILE: &'static [u8] =
        include_bytes!("../../resources/fonts/VT323/VT323-Regular.ttf");

    fn draw_memory(
        font: &Font,
        chip8: &Chip8,
        screen_dims: Vector2,
        handle: &mut raylib::prelude::RaylibDrawHandle,
    ) {
        let text = print_memory(chip8);
        handle.draw_rectangle_v(
            times(vec2!(Self::DEBUG_MEMORY_WINDOW), screen_dims),
            times(
                vec2!(
                    Self::DEBUG_MEMORY_WINDOW.width,
                    Self::DEBUG_MEMORY_WINDOW.height
                ),
                screen_dims,
            ),
            Color::LIGHTGRAY,
        );
        handle.draw_text_ex(
            font,
            &text,
            vec2!(
                (screen_dims.x * Self::DEBUG_MEMORY_WINDOW.x) as i32 + 5,
                (screen_dims.y * Self::DEBUG_MEMORY_WINDOW.y) as i32 + 10
            ),
            18.0,
            1.0,
            Color::BLACK,
        );
    }
    fn draw_registers(
        chip8: &Chip8,
        screen_dims: Vector2,
        handle: &mut raylib::prelude::RaylibDrawHandle,
    ) {
        let text = print_registers(chip8);
        handle.draw_rectangle_v(
            times(vec2!(Self::DEBUG_REGISTER_WINDOW), screen_dims),
            times(
                vec2!(
                    Self::DEBUG_REGISTER_WINDOW.width,
                    Self::DEBUG_REGISTER_WINDOW.height
                ),
                screen_dims,
            ),
            Color::DARKGRAY,
        );
        handle.draw_text(
            &text,
            (screen_dims.x * Self::DEBUG_REGISTER_WINDOW.x) as i32 + 5,
            (screen_dims.y * Self::DEBUG_REGISTER_WINDOW.y) as i32 + 10,
            18,
            Color::WHITE,
        );
    }
}

impl Default for RaylibDisplay {
    fn default() -> Self {
        let (mut rhandle, rthread) = RaylibBuilder::default()
            .width(Self::WINDOW_WIDTH)
            .height(Self::WINDOW_HEIGHT)
            .resizable()
            .title("Chip-8")
            .build();

        let keymap: HashMap<KeyboardKey, KeyInput> =
            HashMap::from_iter(Self::KEYMAP.iter().copied());
        let keys_down: Vec<(KeyboardKey, KeyState)> = Vec::from_iter(
            Self::KEYMAP
                .iter()
                .copied()
                .map(|(key, _)| (key, KeyState::Up)),
        );

        rhandle.set_text_line_spacing(RaylibInstructionWindow::LINE_SPACING);

        let instruction_window = RaylibInstructionWindow {
            window: super::InstructionWindow::default(),
            position: Rectangle {
                x: 0.0,
                y: Self::WINDOW_HEIGHT as f32 * Self::DEBUG_INSTRUCTION_WINDOW.y,
                width: Self::WINDOW_WIDTH as f32 * Self::DEBUG_INSTRUCTION_WINDOW.width,
                height: Self::WINDOW_HEIGHT as f32 * Self::DEBUG_INSTRUCTION_WINDOW.height,
            },
        };
        static AUDIO: LazyLock<RaylibAudio> =
            LazyLock::new(|| RaylibAudio::init_audio_device().unwrap());
        let wave = AUDIO
            .new_wave_from_memory("ogg", RaylibDisplay::SOUND_FILE)
            .ok();
        let sound = wave.map(|w| AUDIO.new_sound_from_wave(&w).unwrap());

        let font = rhandle
            .load_font(&rthread, "resources/fonts/VT323/VT323-Regular.ttf")
            .unwrap();
        Self {
            raylib_handle: rhandle,
            raylib_thread: rthread,
            raylib_sound: sound,
            keymap,
            font: Some(font),
            debug_mode: false,
            keys_down,
            instruction_window,
            breakpoints: BitArray::ZERO,
        }
    }
}

fn times(v1: Vector2, v2: Vector2) -> Vector2 {
    vec2!(v1.x * v2.x, v1.y * v2.y)
}

impl RaylibDisplay {
    fn update(&mut self, chip8: &Chip8, show_current_instruction: bool) -> bool {
        self.keys_down = self
            .keys_down
            .iter()
            .map(|(key, state)| {
                (
                    *key,
                    match (self.raylib_handle.is_key_down(*key), state) {
                        (true, KeyState::Up) => KeyState::Pressed,
                        (true, KeyState::Pressed) => KeyState::HeldSince(time::Instant::now()),
                        (true, held_since) => *held_since,
                        (false, _) => KeyState::Up,
                    },
                )
            })
            .collect();

        let screen_width = self.raylib_handle.get_screen_width();
        let pixel_width: i32 = ((screen_width / crate::DISPLAY_COLUMNS as i32) as f32
            * (if self.debug_mode {
                Self::DEBUG_MAIN_WINDOW.width
            } else {
                1.0
            })) as i32;
        let screen_height = self.raylib_handle.get_screen_height();
        let pixel_height = ((screen_height / crate::DISPLAY_ROWS as i32) as f32
            * (if self.debug_mode {
                Self::DEBUG_MAIN_WINDOW.height
            } else {
                1.0
            })) as i32;
        {
            let mut handle = self.raylib_handle.begin_drawing(&self.raylib_thread);
            handle.clear_background(Color::BLACK);
            for x in 0..crate::DISPLAY_COLUMNS {
                for y in 0..crate::DISPLAY_ROWS {
                    let pixel = chip8.memory.display[[x, y]];
                    if pixel {
                        handle.draw_rectangle(
                            x as i32 * pixel_width,
                            y as i32 * pixel_height,
                            pixel_width,
                            pixel_height,
                            Color::WHITE,
                        )
                    }
                }
            }
            if self.debug_mode {
                let screen_dims = vec2!(screen_width, screen_height);
                // Draw instructions
                self.instruction_window.refresh_position(screen_dims);
                if show_current_instruction {
                    self.instruction_window.window.focus(chip8.pc());
                }
                self.instruction_window.draw(
                    &self.font.as_ref().unwrap(),
                    &self.breakpoints,
                    chip8,
                    &mut handle,
                );
                // Draw memory view
                Self::draw_memory(self.font.as_ref().unwrap(), chip8, screen_dims, &mut handle);

                // Draw register view
                Self::draw_registers(chip8, screen_dims, &mut handle);
            }
        }
        if let Some(sound) = self.raylib_sound.as_mut() {
            if sound.is_playing() & !chip8.sound() {
                sound.stop();
            }
            if !sound.is_playing() & chip8.sound() {
                sound.play();
            }
        }

        self.raylib_handle.window_should_close()
    }

    fn get_inputs(&mut self) -> Vec<KeyInput> {
        let delay = Duration::from_millis(250);
        let now = time::Instant::now();
        let mut inputs = self
            .keys_down
            .iter()
            .filter_map(|(key, state)| match state {
                KeyState::HeldSince(t) => {
                    if now - *t > delay {
                        Some(self.keymap[key])
                    } else {
                        None
                    }
                }
                KeyState::Up => None,
                KeyState::Pressed => Some(self.keymap[key]),
            })
            .collect_vec();
        if self
            .raylib_handle
            .is_mouse_button_pressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT)
        {
            inputs.push(KeyInput::Click(
                vec2!(
                    self.raylib_handle.get_mouse_x() as f32,
                    self.raylib_handle.get_mouse_y() as f32
                )
                .into(),
            ));
        }
        let mouse_wheel = self.raylib_handle.get_mouse_wheel_move().round() as isize;
        if mouse_wheel != 0 {
            // negative is down, but everywhere else negative is up, so we invert the scroll amount to match
            inputs.push(KeyInput::Scroll(
                vec2!(
                    self.raylib_handle.get_mouse_x(),
                    self.raylib_handle.get_mouse_y()
                )
                .into(),
                -mouse_wheel,
            ))
        }
        inputs
    }

    fn toggle_debug(&mut self) {
        self.debug_mode = !self.debug_mode;
    }

    fn on_mouse_click(&mut self, position: super::Vector) {
        let screen_dims = vec2!(
            self.raylib_handle.get_screen_width(),
            self.raylib_handle.get_screen_height()
        );
        match (
            ((position.x / screen_dims.x) < 0.5),
            ((position.y / screen_dims.y) < 0.5),
        ) {
            (true, true) => { // chip8 window
            }
            (true, false) => {
                if let Some(addr) = self.instruction_window.get_addr(position.y) {
                    let prev = *self.breakpoints.get(addr).unwrap();
                    self.breakpoints.set(addr, !prev);
                }
            } //  instruction view
            (false, true) => {}  //  memory view
            (false, false) => {} // register view
        }
    }

    fn on_mouse_scroll(&mut self, position: super::Vector, direction: isize) {
        let screen_dims = vec2!(
            self.raylib_handle.get_screen_width(),
            self.raylib_handle.get_screen_height()
        );
        match (
            ((position.x / screen_dims.x) < 0.5),
            ((position.y / screen_dims.y) < 0.5),
        ) {
            (true, true) => { // chip8 window
            }
            (true, false) => {
                self.instruction_window
                    .window
                    .scroll(direction * (INSTRUCTION_SIZE as isize));
            } //  instruction view
            (false, true) => {}  //  memory view
            (false, false) => {} // register view
        }
    }

    fn is_breakpoint(&self, addr: usize) -> bool {
        return *self
            .instruction_window
            .window
            .breakpoints
            .get(addr)
            .as_deref()
            .unwrap_or(&false);
    }
}

struct RaylibInstructionWindow {
    window: super::InstructionWindow,
    position: Rectangle,
}

impl RaylibInstructionWindow {
    const LINE_SPACING: i32 = 20;
    const MARGIN_TOP: f32 = 15.0;
    const MARGIN_BOTTOM: f32 = 25.0;
    const MARGIN_LEFT: f32 = 50.0;

    fn line_height(&self) -> f32 {
        (self.position.height - (Self::MARGIN_TOP + Self::MARGIN_BOTTOM)) / self.window.len as f32
    }

    fn grid_line(&self, lineno: usize) -> f32 {
        self.position.y + Self::MARGIN_TOP + (lineno as f32) * self.line_height()
    }

    pub(crate) fn draw<T: RaylibDraw>(
        &self,
        font: &Font,
        breakpoints: &BitArr!(for MEMORY_SIZE),
        chip8: &Chip8,
        handle: &mut T,
    ) {
        handle.draw_rectangle_v(
            vec2!(self.position.x, self.position.y),
            vec2!(self.position.width, self.position.height),
            Color::WHITE,
        );
        for (i, (addr, line)) in self.window.lines(chip8).iter().enumerate() {
            if *breakpoints.get(*addr).as_deref().unwrap_or(&false) {
                handle.draw_circle(
                    (self.line_height() / 2.0) as i32,
                    (self.grid_line(i) + self.line_height() / 2.0) as i32,
                    self.line_height() / 4.0,
                    Color::RED,
                );
            }
            handle.draw_text_ex(
                font,
                line,
                vec2!(Self::MARGIN_LEFT, self.grid_line(i)),
                32.0,
                1.0,
                Color::BLACK,
            );
        }
    }

    pub(crate) fn get_addr(&self, y: f32) -> Option<usize> {
        let offset = y - self.position.y - Self::MARGIN_TOP;
        if offset.is_sign_negative() {
            None
        } else {
            let line_height = (self.position.height - (Self::MARGIN_TOP + Self::MARGIN_BOTTOM))
                / self.window.len as f32;
            let line_no = (offset / line_height).trunc() as usize;
            Some((line_no * INSTRUCTION_SIZE) + self.window.start_addr)
        }
    }

    fn refresh_position(&mut self, screen_dims: Vector2) {
        let xy = times(screen_dims, vec2!(RaylibDisplay::DEBUG_INSTRUCTION_WINDOW));
        let wh = times(
            screen_dims,
            vec2!(
                RaylibDisplay::DEBUG_INSTRUCTION_WINDOW.width,
                RaylibDisplay::DEBUG_INSTRUCTION_WINDOW.height
            ),
        );
        self.position = Rectangle {
            x: xy.x,
            y: xy.y,
            width: wh.x,
            height: wh.y,
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
