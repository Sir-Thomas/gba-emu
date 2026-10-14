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

#[derive(Clone, Copy, Error, Debug)]
pub enum PpuError {
    #[error("Invalid PPU Mode: {mode}")]
    InvalidMode { mode: u16 },
    #[error("Invalid Index")]
    IndexError,
}

#[derive(Debug, Default, Clone, Copy)]
enum Mode {
    #[default]
    Mode0,
    Mode1,
    Mode2,
    Mode3,
    Mode4,
    Mode5,
}

pub struct Ppu {
    mode: Mode,
    framebuffer: Vec<u16>,
    frame_select: bool,
    tick: usize,
    scanline: usize,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            framebuffer: vec![0; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            frame_select: false,
            tick: 0,
            scanline: 0,
        }
    }
}

impl Ppu {
    pub fn framebuffer(&self) -> &[u16] {
        &self.framebuffer
    }

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

    pub fn tick(&mut self, bus: &mut Bus) -> Result<(bool, bool), PpuError> {
        let mut blanks = (false, false);
        if self.tick == 960 && self.scanline < 160 {
            self.scanline(bus)?;
        }
        self.tick = self.tick.wrapping_add(1);
        if self.tick == 1006 {
            bus.set_hblank();
            blanks.1 = true;
        } else if self.tick == 1232 {
            self.tick = 0;
            bus.clear_hblank();
            self.scanline = self.scanline.wrapping_add(1) % 228;
            bus.write_16(0x0400_0006, self.scanline.truncate());
            if self.scanline == 227 {
                bus.clear_vblank();
            } else if self.scanline == 160 {
                bus.set_vblank();
                blanks.0 = true;
            }
        }
        Ok(blanks)
    }

    pub fn scanline(&mut self, bus: &mut Bus) -> Result<(), PpuError> {
        self.set_mode(bus)?;
        match self.mode {
            Mode::Mode0 => self.mode0_scanline(self.scanline, bus),
            Mode::Mode1 => self.mode1_scanline(self.scanline, bus),
            Mode::Mode2 => self.mode2_scanline(self.scanline, bus),
            Mode::Mode3 => self.mode3_scanline(self.scanline, bus),
            Mode::Mode4 => self.mode4_scanline(self.scanline, bus),
            Mode::Mode5 => self.mode5_scanline(self.scanline, bus),
        }
    }

    fn mode0_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        // TODO: Implement mode 0
        // This isn't the macro because test roms start in mode 0
        Ok(())
    }

    fn mode1_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        todo!("PPU Mode 1");
        Ok(())
    }

    fn mode2_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        todo!("PPU Mode 2");
        Ok(())
    }

    fn mode3_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        for address in (0..DISPLAY_WIDTH) {
            let a = address.wrapping_add(DISPLAY_WIDTH.wrapping_mul(scanline));
            *self.framebuffer.get_mut(a).ok_or(PpuError::IndexError)? =
                bus.read_16(a.wrapping_mul(2).wrapping_add(VRAM_START_ADDRESS));
        }
        Ok(())
    }

    fn mode4_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        self.set_frame(bus);
        for address in (0..DISPLAY_WIDTH) {
            let a = address.wrapping_add(DISPLAY_WIDTH.wrapping_mul(scanline));
            *self.framebuffer.get_mut(a).ok_or(PpuError::IndexError)? = self.mode4_pixel(a, bus);
        }
        Ok(())
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

    fn mode5_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        todo!("PPU Mode 5");
        Ok(())
    }
}
