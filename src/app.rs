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

#[derive(Debug)]
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

    fn run(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        self.previous_frame = self.current_frame;
        self.current_frame = Instant::now();
        let frame_time = self.current_frame.duration_since(self.previous_frame);
        self.accumulator = self.accumulator.saturating_add(frame_time);
        let mut cycles = 0.0;
        //while self.accumulator > Duration::from_secs_f64(CYCLE_TIME) {
        //self.gba.cpu_cycle();
        //self.accumulator = self
        //.accumulator
        //.saturating_sub(Duration::from_secs_f64(CYCLE_TIME));
        //cycles += 1.0;
        //}

        // This gets things working for now. I'll need to move this into the ppu to run ROMs that
        // need to update graphics mid-frame.
        for scanline in 0..228 {
            if scanline == 0 {
                self.gba.clear_vblank();
            } else if scanline == 160 {
                self.gba.set_vblank();
            }
            for cycle in 0..1232 {
                if cycle == 0 {
                    self.gba.clear_hblank();
                } else if cycle == 960 {
                    self.gba.set_hblank();
                }
                self.gba.cpu_cycle();
            }
        }
        let image = framebuffer_to_image(self.gba.draw());
        self.display_texture.set(image, TextureOptions::NEAREST);
        CentralPanel::default().show(ui, |ui| {
            let (opcode, instruction) = self.gba.next_instruction();
            ui.label(format!("Next Instruction: {opcode:#06X} {instruction:?}"));
            ui.label(format!(
                "Program Counter: {:#010X}",
                self.gba.program_counter()
            ));
            ui.label(format!("Link Register: {:#010X}", self.gba.link_register()));
            ui.label(format!("Stack Pointer: {:#010X}", self.gba.stack_pointer()));
            ui.label(format!("R00: {:#010X}", self.gba.register(0)));
            ui.label(format!("R01: {:#010X}", self.gba.register(1)));
            ui.label(format!("R02: {:#010X}", self.gba.register(2)));
            ui.label(format!("R03: {:#010X}", self.gba.register(3)));
            ui.label(format!("R04: {:#010X}", self.gba.register(4)));
            ui.label(format!("R05: {:#010X}", self.gba.register(5)));
            ui.label(format!("R06: {:#010X}", self.gba.register(6)));
            ui.label(format!("R07: {:#010X}", self.gba.register(7)));
            ui.label(format!("R08: {:#010X}", self.gba.register(8)));
            ui.label(format!("R09: {:#010X}", self.gba.register(9)));
            ui.label(format!("R10: {:#010X}", self.gba.register(10)));
            ui.label(format!("R11: {:#010X}", self.gba.register(11)));
            ui.label(format!("R12: {:#010X}", self.gba.register(12)));
            ui.label(format!("R13: {:#010X}", self.gba.register(13)));
            ui.label(format!("R14: {:#010X}", self.gba.register(14)));
            ui.label(format!("R15: {:#010X}", self.gba.register(15)));
            ui.label(format!(
                "N: {:?} Z: {:?} C: {:?} V: {:?}",
                self.gba.negative(),
                self.gba.zero(),
                self.gba.carry(),
                self.gba.overflow()
            ));
            ui.add(Image::new(&self.display_texture).fit_to_original_size(self.scale));
        });
        ui.request_repaint();
    }

    fn debug(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        CentralPanel::default().show(ui, |ui| {
            ui.label(format!("Mode: {:?}", self.mode));
            if ui.add(Button::new("advance")).clicked() {
                self.gba.cpu_cycle();
                let image = framebuffer_to_image(self.gba.draw());
                self.display_texture.set(image, TextureOptions::NEAREST);
            }
            if ui.add(Button::new("run")).clicked() {
                self.mode = Mode::Run;
                ui.request_repaint();
            }
            let (opcode, instruction) = self.gba.next_instruction();
            ui.label(format!("Next Instruction: {opcode:#06X} {instruction:?}"));
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
                "Program Counter: {:#010X}",
                self.gba.program_counter()
            ));
            ui.label(format!("Link Register: {:#010X}", self.gba.link_register()));
            ui.label(format!("Stack Pointer: {:#010X}", self.gba.stack_pointer()));
            ui.label(format!("R00: {:#010X}", self.gba.register(0)));
            ui.label(format!("R01: {:#010X}", self.gba.register(1)));
            ui.label(format!("R02: {:#010X}", self.gba.register(2)));
            ui.label(format!("R03: {:#010X}", self.gba.register(3)));
            ui.label(format!("R04: {:#010X}", self.gba.register(4)));
            ui.label(format!("R05: {:#010X}", self.gba.register(5)));
            ui.label(format!("R06: {:#010X}", self.gba.register(6)));
            ui.label(format!("R07: {:#010X}", self.gba.register(7)));
            ui.label(format!("R08: {:#010X}", self.gba.register(8)));
            ui.label(format!("R09: {:#010X}", self.gba.register(9)));
            ui.label(format!("R10: {:#010X}", self.gba.register(10)));
            ui.label(format!("R11: {:#010X}", self.gba.register(11)));
            ui.label(format!("R12: {:#010X}", self.gba.register(12)));
            ui.label(format!("R13: {:#010X}", self.gba.register(13)));
            ui.label(format!("R14: {:#010X}", self.gba.register(14)));
            ui.label(format!("R15: {:#010X}", self.gba.register(15)));
            ui.label(format!(
                "N: {:?} Z: {:?} C: {:?} V: {:?}",
                self.gba.negative(),
                self.gba.zero(),
                self.gba.carry(),
                self.gba.overflow()
            ));
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
