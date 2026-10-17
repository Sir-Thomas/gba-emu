#![allow(unused)]

use bitfields::{bitfield, bitflag};
use thiserror::Error;

use crate::bus::Bus;
use crate::gba::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
use crate::ppu::Mode::Mode3;

const VRAM_START_ADDRESS: usize = 0x0600_0000;
const FRAME0_START: usize = 0x0600_0000;
const FRAME1_START: usize = 0x0600_A000;
const DISPLAY_CONTROL: usize = 0x0400_0000;
const BACKGROUND_CONTROL_0: usize = 0x0400_0008;
const BACKGROUND_CONTROL_1: usize = 0x0400_000A;
const BACKGROUND_CONTROL_2: usize = 0x0400_000C;
const BACKGROUND_CONTROL_3: usize = 0x0400_000E;
pub const DISPSTAT: usize = 0x0400_0004;
const PALETTE: usize = 0x0500_0000;

#[derive(Clone, Copy, Error, Debug)]
pub enum PpuError {
    #[error("Invalid PPU Mode: {mode}")]
    InvalidMode { mode: u16 },
    #[error("Invalid Index")]
    IndexError,
}

pub struct Ppu {
    tick: usize,
    scanline: usize,
    framebuffer: Vec<u16>,
    display_control: DisplayControl,
    backgrounds: [Background; 4],
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            tick: 0,
            scanline: 0,
            framebuffer: vec![0; DISPLAY_WIDTH * DISPLAY_HEIGHT],
        }
    }
}

impl Ppu {
    pub fn framebuffer(&self) -> &[u16] {
        &self.framebuffer
    }

    pub fn tick(&mut self, bus: &mut Bus) -> Result<(bool, bool), PpuError> {
        let mut blanks = (false, false);
        if self.tick == 960 && self.scanline < 160 {
            self.update(bus);
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

    fn update(&mut self, bus: &Bus) {
        self.display_control = bus.read_16(DISPLAY_CONTROL).into();
        self.background_control_0 = bus.read_16(BACKGROUND_CONTROL_0).into();
    }

    fn scanline(&mut self, bus: &mut Bus) -> Result<(), PpuError> {
        match self.display_control.mode() {
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
        for address in (0..DISPLAY_WIDTH) {
            let a = address.wrapping_add(DISPLAY_WIDTH.wrapping_mul(scanline));
            *self.framebuffer.get_mut(a).ok_or(PpuError::IndexError)? = self.mode4_pixel(a, bus);
        }
        Ok(())
    }

    fn mode4_pixel(&self, address: usize, bus: &Bus) -> u16 {
        let start_address = if self.display_control.frame_select() {
            FRAME1_START
        } else {
            FRAME0_START
        };
        let pixel_data = bus.read_8(address.wrapping_add(start_address));
        bus.read_16(PALETTE.wrapping_add(usize::from(pixel_data).wrapping_mul(2)))
    }

    fn mode5_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        todo!("PPU Mode 5");
        Ok(())
    }
}

#[bitfield(u16)]
#[derive(Hash)]
struct DisplayControl {
    #[bits(3)]
    mode: Mode,
    _reserved: bool,
    frame_select: bool,
    allow_oam_access_in_hblank: bool,
    one_dimensional_obj_character_vram_mapping: bool,
    forced_blank: bool,
    display_bg_0: bool,
    display_bg_1: bool,
    display_bg_2: bool,
    display_bg_3: bool,
    display_obj: bool,
    window_0: bool,
    window_1: bool,
    obj_window: bool,
}

#[bitflag(u8)]
#[derive(Default, Debug)]
enum Mode {
    #[base]
    #[default]
    Mode0 = 0,
    Mode1 = 1,
    Mode2 = 2,
    Mode3 = 3,
    Mode4 = 4,
    Mode5 = 5,
}

struct Background {
    control_address: usize,
    control: BackgroundControl,
}

#[bitflag(u8)]
enum ColorsPaletts {
    #[base]
    Use16Colors16Palettes = 0,
    Use256Colors1Palette = 1,
}

#[bitflag(u8)]
enum Overflow {
    #[base]
    Transparent = 0,
    Wraparound = 1,
}

#[bitflag(u8)]
enum ScreenSize {
    #[base]
    Size0 = 0,
    Size1 = 1,
    Size2 = 2,
    Size3 = 3,
}

#[bitfield(u16)]
#[derive(Hash)]
struct BackgroundControl {
    #[bits(2)]
    priority: u8,
    #[bits(2)]
    character_base: u32,
    #[bits(2)]
    _reserved: u8,
    mosaic: bool,
    #[bits(1)]
    colors_palettes: ColorsPaletts,
    #[bits(5)]
    screen_base: u32,
    #[bits(1)]
    overflow: Overflow,
    #[bits(2)]
    screen_size: ScreenSize,
}
