use std::{ops::Deref, process::exit};

use crate::{driver::{Chip8Driver, EmulatorMode}, emulator::MEMORY_SIZE, Chip8, DISPLAY_COLUMNS, DISPLAY_ROWS};
use bitvec::{array::BitArray, BitArr};
use egui::{ahash::HashMap, pos2, vec2, Color32, Key, Rect, Response, Rounding, Sense, Ui, Vec2};
use egui_miniquad::EguiMq;
use itertools::Itertools;
use miniquad as mq;

use super::{print_memory, print_registers, InstructionWindow, KeyInput, Vector};

pub struct EguiDriver {
    chip8: Chip8,
    display: EguiDisplay,
    mode: EmulatorMode,
    mq_context: Box<dyn mq::RenderingBackend>,
    egui_mq: egui_miniquad::EguiMq,
}

impl EguiDriver{


    pub fn new(speed: Option<u64>, paused: bool) -> Self {
        let mut mq_context = mq::window::new_rendering_backend();
        Self {
            chip8: Chip8::init(speed),
            display: EguiDisplay::default(),
            mode: if paused {EmulatorMode::Paused} else {EmulatorMode::Running},
            egui_mq: egui_miniquad::EguiMq::new(&mut *mq_context),
            mq_context,

        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.chip8.load_rom(rom)
    }
    
    pub fn step_paused(&mut self) {
        let mut toggle_debug = false;
        let mut click_position: Option<Vector> = None;
        let mut scroll_position: Option<Vector> = None;
        let mut scroll_amount = 0;

        for k in &self.display.inputs{
            match k {
                KeyInput::Step => {
                    self.chip8.do_instruction();
                    self.chip8.tick_timers();
                }
                KeyInput::Chip8Key(val) => {
                    self.chip8.clear_keys();
                    self.chip8.set_key(*val)
                }
                KeyInput::TogglePause => self.mode = EmulatorMode::Running,
                KeyInput::ToggleDebug => {toggle_debug = true;},
                KeyInput::Click(position) => {click_position = Some(*position)},
                KeyInput::Scroll(position, amount) => {
                    scroll_position = Some(*position);
                    scroll_amount = *amount
                }
            }
        }
        if toggle_debug {
            self.display.toggle_debug();
        }

        if let Some(position) = click_position {
            self.display.on_mouse_click(position);
        }

        if let Some(position) = scroll_position {
            self.display.on_mouse_scroll(position, scroll_amount);
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
            let mut debug_pressed = false;
            for k in &self.display.inputs {
                match k {
                    KeyInput::Chip8Key(key) => self.chip8.set_key(*key),
                    KeyInput::Step => {},
                    KeyInput::TogglePause => {
                        self.mode = EmulatorMode::Paused;
                        break;},
                    KeyInput::ToggleDebug => {debug_pressed = true;}
                    _ => {}
                }
            }
            if debug_pressed {
                self.display.toggle_debug();
            }
            if matches!(self.mode, EmulatorMode::Running) {
                self.chip8.do_instruction();
            }
            if self.display.is_breakpoint(self.chip8.pc()) {
                self.mode = EmulatorMode::Paused;
            }
        }
    }

    pub fn step(&mut self) {
        match self.mode {
            EmulatorMode::Paused => self.step_paused(),
            EmulatorMode::Running => self.step_running(),
        };
    }


}


pub struct EguiDisplay {
    keymap: HashMap<Key, KeyInput>,
    inputs: Vec<KeyInput>,
    debug: bool,
    breakpoints: BitArr!(for MEMORY_SIZE),
    instruction_window: InstructionWindow,
}

impl mq::EventHandler for EguiDriver {
    fn update(&mut self) {
        self.step();
    }

    fn draw(&mut self) {
        self.mq_context.clear(Some((1., 1., 1., 1.)), None, None);
        self.mq_context
            .begin_default_pass(mq::PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.mq_context.end_render_pass();

        self.display.update(&self.chip8,&mut self.egui_mq, &mut *self.mq_context, matches!(self.mode, EmulatorMode::Running));
    }
    
    fn window_minimized_event(&mut self) {
        self.mode = EmulatorMode::Paused;
    }
    
    fn window_restored_event(&mut self) {
        self.mode = EmulatorMode::Running;
    }
    
    fn quit_requested_event(&mut self) {
        exit(0);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(x, y);
    }

    fn mouse_wheel_event(&mut self, dx: f32, dy: f32) {
        self.egui_mq.mouse_wheel_event(dx, dy);
    }

    fn mouse_button_down_event(
        &mut self,
        mb: mq::MouseButton,
        x: f32,
        y: f32,
    ) {
        self.egui_mq.mouse_button_down_event(mb, x, y);
    }

    fn mouse_button_up_event(
        &mut self,
        mb: mq::MouseButton,
        x: f32,
        y: f32,
    ) {
        self.egui_mq.mouse_button_up_event(mb, x, y);
    }

    fn char_event(
        &mut self,
        character: char,
        _keymods: mq::KeyMods,
        _repeat: bool,
    ) {
        self.egui_mq.char_event(character);
    }

    fn key_down_event(
        &mut self,
        keycode: mq::KeyCode,
        keymods: mq::KeyMods,
        _repeat: bool,
    ) {
        self.egui_mq.key_down_event(keycode, keymods);
    }

    fn key_up_event(&mut self, keycode: mq::KeyCode, keymods: mq::KeyMods) {
        self.egui_mq.key_up_event(keycode, keymods);
    }
}

impl Chip8Driver for EguiDriver {
    fn run(rom: &[u8], speed: Option<u64>, paused: bool) {
        let conf = mq::conf::Conf::default();
        let rom = Vec::from_iter(rom.iter().cloned());
        mq::start(conf, move|| {
            let mut driver = Self::new(speed, paused);
            driver.load_rom(&rom);
            Box::new(driver)
        });
    }
}


impl EguiDisplay {
    const KEYMAP: [(Key, KeyInput); 20] = [
        (Key::Num1, KeyInput::Chip8Key(0x1)),
        (Key::Num2, KeyInput::Chip8Key(0x2)),
        (Key::Num3, KeyInput::Chip8Key(0x3)),
        (Key::Num4, KeyInput::Chip8Key(0xc)),
        (Key::Q, KeyInput::Chip8Key(0x4)),
        (Key::W, KeyInput::Chip8Key(0x5)),
        (Key::E, KeyInput::Chip8Key(0x6)),
        (Key::R, KeyInput::Chip8Key(0xd)),
        (Key::A, KeyInput::Chip8Key(0x7)),
        (Key::S, KeyInput::Chip8Key(0x8)),
        (Key::D, KeyInput::Chip8Key(0x9)),
        (Key::F, KeyInput::Chip8Key(0xe)),
        (Key::Z, KeyInput::Chip8Key(0xa)),
        (Key::X, KeyInput::Chip8Key(0x0)),
        (Key::C, KeyInput::Chip8Key(0xb)),
        (Key::V, KeyInput::Chip8Key(0xf)),
        (Key::Space, KeyInput::TogglePause),
        (Key::P, KeyInput::TogglePause),
        (Key::Period, KeyInput::ToggleDebug),
        (Key::Enter, KeyInput::Step),
    ];

    fn draw_screen(chip8: &Chip8) -> impl FnOnce(&mut Ui) -> Response {
        let colors = chip8
            .memory
            .display
            .outer_iter()
            .map(|row| {
                row.iter()
                    .map(|cell| Color32::from_gray(if *cell { u8::MAX } else { 0 }))
                    .collect_vec()
            })
            .collect_vec();
        move |ui: &mut Ui| {
            let height = ui.available_height();
            let width = ui.available_width();
            let pixel_height = height / DISPLAY_ROWS as f32;
            let pixel_width = width / DISPLAY_COLUMNS as f32;
            let draw_pixel = |x,y,shade: &Color32|{
                ui.painter().rect_filled(Rect::from_min_size(pos2(x,y), vec2(pixel_width,pixel_height)), 
                Rounding::ZERO,
            *shade)
            };
            for (x,row) in colors.iter().enumerate(){
                for (y, color) in row.iter().enumerate(){
                    draw_pixel(x as f32 * pixel_width, y as f32 * pixel_height, color);
                }
            }
            let (_, response) = ui.allocate_at_least(Vec2::ZERO, Sense::focusable_noninteractive());
            response
        }
    }

    fn draw_memory(chip8: &Chip8) -> impl FnOnce(&mut Ui) -> Response {
        let text = print_memory(chip8);
        move |ui| ui.label(text)
    }

    fn draw_registers(chip8: &Chip8) -> impl FnOnce(&mut Ui) -> Response {
        let text = print_registers(chip8);
        move |ui| ui.label(text)
    }

    fn draw_instructions(
        window: &InstructionWindow,
        chip8: &Chip8,
    ) -> impl FnOnce(&mut Ui) -> Response {
        let lines = window.lines(chip8);
        move |ui| ui.label(lines.iter().map(|(_addr, text)| text).join("\n"))
    }
}

impl Default for EguiDisplay {
    fn default() -> Self {
        Self {
            keymap: HashMap::from_iter(Self::KEYMAP.iter().copied()),
            inputs: vec![],
            debug: false,
            instruction_window: InstructionWindow::default(),
            breakpoints: BitArray::ZERO,
        }
    }
}

impl EguiDisplay {
    fn update(&mut self, chip8: &crate::Chip8, egui_mq: &mut EguiMq, mq_context: &mut mq::Context, show_current_instruction: bool) {
        if show_current_instruction {
            self.instruction_window.focus(chip8.pc());
        }
        egui_mq
            .run(&mut *mq_context, |_mq_ctx, egui_ctx| {
                self.inputs = egui_ctx
                .input(|i| i.keys_down.iter().map(|k| self.keymap[k]).collect());
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    if self.debug {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.add(Self::draw_screen(chip8))
                                    | ui.add(Self::draw_instructions(
                                        &self.instruction_window,
                                        chip8,
                                    ))
                            })
                            .response
                                | (ui.vertical(|ui| {
                                    ui.add(Self::draw_memory(chip8))
                                        | ui.add(Self::draw_registers(chip8))
                                }))
                                .response
                        })
                        .inner
                    } else {
                        ui.add(Self::draw_screen(chip8))
                    }
                });
            });
    }

    fn toggle_debug(&mut self) {
        self.debug = !self.debug
    }

    fn is_breakpoint(&self, addr: usize) -> bool {
        // TODO: IMPLEMENT THIS
        false
    }

    fn on_mouse_scroll(&mut self, position: super::Vector, direction: isize) {
        todo!()
    }

    fn on_mouse_click(&mut self, position: super::Vector) {}
    
}
