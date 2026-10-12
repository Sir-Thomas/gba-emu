#![allow(unused)]

use thiserror::Error;

use crate::bus::Bus;
use crate::gba::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
use crate::ppu::Mode::Mode3;

const VRAM_START_ADDRESS: usize = 0x0600_0000;
const FRAME0_START: usize = 0x0600_0000;
const FRAME1_START: usize = 0x0600_A000;
const DISPLAY_CONTROL: usize = 0x0400_0000;
pub const DISPSTAT: usize = 0x0400_0004;
const PALETTE: usize = 0x0500_0000;

#[derive(Error, Debug)]
pub enum PpuError {
    #[error("Invalid PPU Mode: {mode}")]
    InvalidMode { mode: u16 },
    #[error("Invalid Index")]
    IndexError,
}

#[derive(Debug, Default, Clone, Copy)]
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
    frame_select: bool,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            framebuffer: vec![0; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            frame_select: false,
        }
    }
}

impl Ppu {
    fn set_mode(&mut self, bus: &Bus) -> Result<(), PpuError> {
        const MODE_BITS: u16 = 0x7;
        let mode_bits = bus.read_16(DISPLAY_CONTROL) & MODE_BITS;
        self.mode = match mode_bits {
            0 => Mode::Mode0,
            1 => Mode::Mode1,
            2 => Mode::Mode2,
            3 => Mode::Mode3,
            4 => Mode::Mode4,
            5 => Mode::Mode5,
            _ => return Err(PpuError::InvalidMode { mode: mode_bits }),
        };
        Ok(())
    }

    fn set_frame(&mut self, bus: &Bus) {
        const FRAME_SELECT_BIT: u16 = 1 << 4;
        self.frame_select = bus.read_16(DISPLAY_CONTROL) & FRAME_SELECT_BIT > 0;
    }

    pub fn draw(&mut self, bus: &mut Bus) -> Result<&[u16], PpuError> {
        self.set_mode(bus);
        match self.mode {
            Mode::Mode0 => self.draw_mode3(bus),
            Mode::Mode3 => self.draw_mode3(bus),
            Mode::Mode4 => self.draw_mode4(bus),
            _ => todo!("Implement remaining display modes {:?}", self.mode),
        }
    }

    fn draw_mode3(&mut self, bus: &Bus) -> Result<&[u16], PpuError> {
        for address in (0..DISPLAY_WIDTH * DISPLAY_HEIGHT) {
            *self
                .framebuffer
                .get_mut(address)
                .ok_or(PpuError::IndexError)? =
                bus.read_16(address.wrapping_mul(2).wrapping_add(VRAM_START_ADDRESS));
        }
        Ok(&self.framebuffer)
    }

    fn draw_mode4(&mut self, bus: &Bus) -> Result<&[u16], PpuError> {
        self.set_frame(bus);
        for address in (0..DISPLAY_WIDTH * DISPLAY_HEIGHT) {
            *self
                .framebuffer
                .get_mut(address)
                .ok_or(PpuError::IndexError)? = self.mode4_pixel(address, bus);
        }
        Ok(&self.framebuffer)
    }

    fn mode4_pixel(&self, address: usize, bus: &Bus) -> u16 {
        let start_address = if !self.frame_select {
            FRAME0_START
        } else {
            FRAME1_START
        };
        let pixel_data = bus.read_8(address.wrapping_add(start_address));
        bus.read_16(PALETTE.wrapping_add(usize::from(pixel_data).wrapping_mul(2)))
    }
}
