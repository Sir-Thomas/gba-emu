#![allow(unused)]

use std::time::{Duration, Instant};

use eframe::{
    CreationContext, Frame,
    egui::{ColorImage, TextureOptions},
};
use egui::{Button, CentralPanel, Color32, Image, TextEdit, Ui};
use moving_avg::MovingAverage;

use crate::gba::{DISPLAY_HEIGHT, DISPLAY_WIDTH, Gba};

const STARTING_SCALE: f32 = 2.0;
const CPU_FREQUENCY: f64 = 16_776_000.0; //16.776MHz (maybe should be 16.78MHz)
const CYCLE_TIME: f64 = 1.0 / CPU_FREQUENCY;
const FRAME_TIME_SAMPLES: usize = 60;

enum Mode {
    Debug,
    Run,
}

pub struct GbaApp {
    gba: Gba,
    display_texture: egui::TextureHandle,
    current_frame: Instant,
    previous_frame: Instant,
    accumulator: Duration,
    frame_time: MovingAverage<f64>,
    mode: Mode,
    text_input: String,
    scale: f32,
}

impl GbaApp {
    pub fn new(cc: &CreationContext<'_>, mut gba: Gba) -> Self {
        let display_texture = cc.egui_ctx.load_texture(
            "gba-display",
            ColorImage::new(
                [DISPLAY_WIDTH, DISPLAY_HEIGHT],
                vec![Color32::BLACK; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            ),
            TextureOptions::NEAREST,
        );

        gba.load_rom();

        Self {
            gba,
            display_texture,
            current_frame: Instant::now(),
            previous_frame: Instant::now(),
            accumulator: Duration::ZERO,
            frame_time: MovingAverage::new(FRAME_TIME_SAMPLES),
            mode: Mode::Debug,
            text_input: "0000".to_owned(),
            scale: STARTING_SCALE,
        }
    }

    fn run(&mut self, ui: &Ui, _frame: &mut Frame) {
        self.previous_frame = self.current_frame;
        self.current_frame = Instant::now();
        let frame_time = self.current_frame.duration_since(self.previous_frame);
        self.accumulator = self.accumulator.saturating_add(frame_time);
        let mut cycles = 0.0;
        while self.accumulator > Duration::from_secs_f64(CYCLE_TIME) {
            self.gba.cpu_cycle();
            self.accumulator = self
                .accumulator
                .saturating_sub(Duration::from_secs_f64(CYCLE_TIME));
            cycles += 1.0;
        }
    }

    fn debug(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        CentralPanel::default().show(ui, |ui| {
            if ui.add(Button::new("advance")).clicked() {
                self.gba.cpu_cycle();
                let image = framebuffer_to_image(self.gba.draw());
                self.display_texture.set(image, TextureOptions::NEAREST);
            }
            let (opcode, instruction) = self.gba.next_instruction();
            ui.label(format!("Next Instruction: {opcode:#010X} {instruction:?}"));
            // if ui.add(Button::new("Insert opcode")).clicked() {
            // if let Ok(value) = opcode.parse() {
            // self.gba.insert_opcode(value);
            // }
            // }
            let response = ui.add(TextEdit::singleline(&mut self.text_input));
            if response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && let Ok(value) = u16::from_str_radix(&self.text_input, 16)
            {
                self.gba.insert_opcode(value);
            }
            ui.label(format!(
                "Program Counter: {:#06X}",
                self.gba.program_counter()
            ));
            ui.label(format!("R00: {:#06X}", self.gba.r00()));
            ui.label(format!("R01: {:#06X}", self.gba.r01()));
            ui.add(Image::new(&self.display_texture).fit_to_original_size(self.scale));
        });
    }
}

impl eframe::App for GbaApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut Frame) {
        match self.mode {
            Mode::Debug => self.debug(ui, frame),
            Mode::Run => self.run(ui, frame),
        }
    }
}

fn framebuffer_to_image(framebuffer: &[u16]) -> ColorImage {
    let pixels = framebuffer
        .iter()
        .map(|pixel| {
            let red = pixel & 0x001F;
            let green = (pixel & 0x03E0) >> 5;
            let blue = (pixel & 0x7C00) >> 10;
            let red = (red * 527 + 23) >> 6;
            let green = (green * 527 + 23) >> 6;
            let blue = (blue * 527 + 23) >> 6;
            Color32::from_rgb(red.truncate(), green.truncate(), blue.truncate())
        })
        .collect();
    ColorImage::new([DISPLAY_WIDTH, DISPLAY_HEIGHT], pixels)
}
