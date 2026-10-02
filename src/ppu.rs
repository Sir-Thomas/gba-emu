use crate::bus::Bus;
use crate::gba::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
use crate::ppu::Mode::Mode3;

const VRAM_START_ADDRESS: usize = 0x0600_0000;

#[derive(Default)]
enum Mode {
    Mode0,
    Mode1,
    Mode2,
    #[default]
    Mode3,
    Mode4,
    Mode5,
}

pub struct Ppu {
    mode: Mode,
    framebuffer: Vec<u16>,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            framebuffer: vec![0; DISPLAY_WIDTH * DISPLAY_HEIGHT],
        }
    }
}

impl Ppu {
    pub fn draw(&mut self, bus: &Bus) -> &Vec<u16> {
        match self.mode {
            Mode3 => self.draw_mode3(bus),
            _ => todo!(),
        }
    }

    fn draw_mode3(&mut self, bus: &Bus) -> &Vec<u16> {
        (0..DISPLAY_WIDTH * DISPLAY_HEIGHT).for_each(|address| {
            self.framebuffer[address] = bus.read_16(address * 2 + VRAM_START_ADDRESS);
        });
        &self.framebuffer
    }
}
