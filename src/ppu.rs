use std::num::TryFromIntError;

use bitfields::{bitfield, bitflag};
use thiserror::Error;

use crate::bus::Bus;
use crate::gba::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

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
    //Not sure how bitfields handles this...
    //#[error("Invalid PPU Mode: {mode}")]
    //InvalidMode { mode: u16 },
    #[error("Invalid Index")]
    IndexError,
    #[error("Could not cast integer")]
    CastError(#[from] TryFromIntError),
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
        let background_0 = Background::new(BACKGROUND_CONTROL_0);
        let background_1 = Background::new(BACKGROUND_CONTROL_1);
        let background_2 = Background::new(BACKGROUND_CONTROL_2);
        let background_3 = Background::new(BACKGROUND_CONTROL_3);
        Self {
            tick: 0,
            scanline: 0,
            framebuffer: vec![0; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            display_control: DisplayControl::default(),
            backgrounds: [background_0, background_1, background_2, background_3],
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

    // If this doesn't end up needing anything more,
    // I should inline the one function into tick.
    fn update(&mut self, bus: &Bus) {
        self.display_control = bus.read_16(DISPLAY_CONTROL).into();
    }

    fn scanline(&mut self, bus: &Bus) -> Result<(), PpuError> {
        let backdrop = bus.read_16(PALETTE);
        let start_index = self.scanline.wrapping_mul(DISPLAY_WIDTH);
        self.framebuffer
            .get_mut(start_index..start_index.wrapping_add(DISPLAY_WIDTH))
            .ok_or(PpuError::IndexError)?
            .fill(backdrop);
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
        if self.display_control.display_bg_0() {
            self.draw_background(0, scanline, bus)?;
        }
        if self.display_control.display_bg_1() {
            self.draw_background(1, scanline, bus)?;
        }
        if self.display_control.display_bg_2() {
            self.draw_background(2, scanline, bus)?;
        }
        if self.display_control.display_bg_3() {
            self.draw_background(3, scanline, bus)?;
        }
        Ok(())
    }

    fn draw_background(
        &mut self,
        background_index: usize,
        scanline: usize,
        bus: &Bus,
    ) -> Result<(), PpuError> {
        let background = self
            .backgrounds
            .get_mut(background_index)
            .ok_or(PpuError::IndexError)?;
        background.update(bus);
        let background = *background; // copy the value out to remove the borrow
        let background_address = usize::try_from(background.control.screen_base())?
            .wrapping_mul(0x800)
            .wrapping_add(VRAM_START_ADDRESS);
        let character_address = usize::try_from(background.control.character_base())?
            .wrapping_mul(0x4000)
            .wrapping_add(VRAM_START_ADDRESS);
        for x in 0usize..240 {
            let background_map = BackgroundMap::from(
                bus.read_16(
                    (x / 8)
                        .wrapping_add((scanline / 8).wrapping_mul(32))
                        .wrapping_mul(2)
                        .wrapping_add(background_address),
                ),
            );
            let (palette, palette_index) = palette_index(
                background.control.colors_palettes(),
                background_map,
                character_address,
                x,
                scanline,
                bus,
            );
            if palette_index != 0 {
                self.set_pixel(x, scanline, get_color(palette, palette_index, bus))?;
            }
        }
        Ok(())
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: u16) -> Result<(), PpuError> {
        *self
            .framebuffer
            .get_mut(x.wrapping_add(y.wrapping_mul(DISPLAY_WIDTH)))
            .ok_or(PpuError::IndexError)? = color;
        Ok(())
    }

    fn mode1_scanline(&self, _scanline: usize, _bus: &Bus) -> Result<(), PpuError> {
        todo!("PPU Mode 1");
    }

    fn mode2_scanline(&self, _scanline: usize, _bus: &Bus) -> Result<(), PpuError> {
        todo!("PPU Mode 2");
    }

    fn mode3_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        for address in 0..DISPLAY_WIDTH {
            let a = address.wrapping_add(DISPLAY_WIDTH.wrapping_mul(scanline));
            *self.framebuffer.get_mut(a).ok_or(PpuError::IndexError)? =
                bus.read_16(a.wrapping_mul(2).wrapping_add(VRAM_START_ADDRESS));
        }
        Ok(())
    }

    fn mode4_scanline(&mut self, scanline: usize, bus: &Bus) -> Result<(), PpuError> {
        for address in 0..DISPLAY_WIDTH {
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

    fn mode5_scanline(&self, _scanline: usize, _bus: &Bus) -> Result<(), PpuError> {
        todo!("PPU Mode 5");
    }
}

fn palette_index(
    colors_palettes: ColorsPaletts,
    background_map: BackgroundMap,
    character_address: usize,
    x: usize,
    y: usize,
    bus: &Bus,
) -> (usize, usize) {
    if colors_palettes == ColorsPaletts::Use16Colors16Palettes {
        let palette = background_map.palette_number().into();
        let palette_indexes = bus.read_8(
            character_address
                .wrapping_add(usize::from(background_map.tile_number()).wrapping_mul(32))
                .wrapping_add((x % 8) / 2)
                .wrapping_add((y % 8).wrapping_mul(4)),
        );
        if x.is_multiple_of(2) {
            (palette, usize::from(palette_indexes & 0x0F))
        } else {
            (palette, usize::from((palette_indexes >> 4) & 0x0F))
        }
    } else {
        (
            0,
            bus.read_8(
                character_address
                    .wrapping_add(usize::from(background_map.tile_number()).wrapping_mul(64))
                    .wrapping_add(x % 8)
                    .wrapping_add((y % 8).wrapping_mul(8)),
            )
            .into(),
        )
    }
}

fn get_color(palette: usize, index: usize, bus: &Bus) -> u16 {
    bus.read_16(
        PALETTE
            .wrapping_add(palette.wrapping_mul(32))
            .wrapping_add(index.wrapping_mul(2)),
    )
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

#[derive(Clone, Copy, Debug)]
struct Background {
    control_address: usize,
    control: BackgroundControl,
}

impl Background {
    fn new(control_address: usize) -> Self {
        Self {
            control_address,
            control: BackgroundControl::default(),
        }
    }

    fn update(&mut self, bus: &Bus) {
        self.control = bus.read_16(self.control_address).into();
    }
}

#[bitflag(u8)]
#[derive(Debug, PartialEq, Eq)]
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

#[bitfield(u16)]
#[derive(Hash)]
struct BackgroundMap {
    #[bits(10)]
    tile_number: u16,
    horizontal_flip: bool,
    vertical_flip: bool,
    #[bits(4)]
    palette_number: u8,
}
