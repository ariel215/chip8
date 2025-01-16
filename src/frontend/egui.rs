use crate::{Chip8, DISPLAY_COLUMNS, DISPLAY_ROWS, MEMORY_SIZE};
use bitvec::{array::BitArray, BitArr};
use egui::{ahash::HashMap, pos2, vec2, Color32, Key, Rect, Response, Rounding, Sense, Ui, Vec2};
use itertools::Itertools;
use miniquad as mq;

use super::{print_memory, print_registers, Chip8Frontend, InstructionWindow, KeyInput};
pub struct EguiDisplay {
    mq_context: Box<dyn mq::RenderingBackend>,
    context: egui::Context,
    egui_mq: egui_miniquad::EguiMq,
    keymap: HashMap<Key, KeyInput>,
    debug: bool,
    breakpoints: BitArr!(for MEMORY_SIZE),
    instruction_window: InstructionWindow,
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
                    .map(|cell| Color32::from_gray(if *cell { 0 } else { 1 }))
                    .collect_vec()
            })
            .collect_vec();
        move |ui: &mut Ui| {
            let height = ui.available_height();
            let width = ui.available_width();
            let pixel_heigt = height / DISPLAY_ROWS as f32;
            let pixel_width = width / DISPLAY_COLUMNS as f32;
            for (x, row) in colors.iter().enumerate() {
                for (y, color) in row.iter().enumerate() {
                    let square = Rect::from_min_size(
                        pos2(x as f32 * pixel_width, y as f32 * pixel_heigt),
                        vec2(pixel_width, pixel_heigt),
                    );
                    ui.painter().rect_filled(square, Rounding::ZERO, *color);
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
        let mut mq_context = mq::window::new_rendering_backend();
        Self {
            context: egui::Context::default(),
            egui_mq: egui_miniquad::EguiMq::new(&mut *mq_context),
            keymap: HashMap::from_iter(Self::KEYMAP.iter().copied()),
            debug: false,
            instruction_window: InstructionWindow::default(),
            breakpoints: BitArray::ZERO,
            mq_context,
        }
    }
}

impl super::Chip8Frontend for EguiDisplay {
    fn update(&mut self, chip8: &crate::Chip8, show_current_instruction: bool) -> bool {
        if show_current_instruction {
            self.instruction_window.focus(chip8.pc());
        }
        self.mq_context.clear(Some((1., 1., 1., 1.)), None, None);
        self.mq_context
            .begin_default_pass(mq::PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.mq_context.end_render_pass();
        
        self.egui_mq
            .run(&mut *self.mq_context, |_mq_ctx, egui_ctx| {
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

        self.egui_mq.draw(&mut *self.mq_context);
        self.mq_context.commit_frame();
        self.context.input(|i| i.viewport().close_requested())
    }

    fn get_inputs(&mut self) -> Vec<super::KeyInput> {
        self.context
            .input(|i| i.keys_down.iter().map(|k| self.keymap[k]).collect())
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
