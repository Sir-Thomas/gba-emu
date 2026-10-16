#![allow(dead_code)]

use std::fs::read;
use thiserror::Error;

use crate::bus::{Bus, GAME_PAK_START_ADDRESS};
use crate::cpu::registers::RegisterError;
use crate::cpu::{Cpu, CpuError, Instruction};
use crate::ppu::{Ppu, PpuError};

pub const DISPLAY_WIDTH: usize = 240;
pub const DISPLAY_HEIGHT: usize = 160;

#[derive(Clone, Error, Debug)]
pub enum GbaError {
    #[error("Cpu Error: {0}")]
    CpuError(#[from] CpuError),
    #[error("Register Error: {0}")]
    RegisterError(#[from] RegisterError),
    #[error("PPU Error: {0}")]
    PpuError(#[from] PpuError),
}

pub struct Gba {
    bus: Bus,
    cpu: Cpu,
    ppu: Ppu,
}

impl Gba {
    pub fn new() -> Self {
        Self {
            bus: Bus::new(),
            cpu: Cpu::new(),
            ppu: Ppu::default(),
        }
    }

    pub fn back(&mut self) {
        self.cpu.back();
    }

    pub fn tick(&mut self) -> Result<(bool, bool), GbaError> {
        self.cpu.cpu_cycle(&mut self.bus)?;
        self.ppu.tick(&mut self.bus).map_err(Into::into)
    }

    pub fn next_instruction(&self) -> Result<(u32, Instruction), GbaError> {
        self.cpu.next_instruction(&self.bus).map_err(Into::into)
    }

    pub fn insert_opcode(&mut self, value: u16) {
        self.bus
            .write_16(self.cpu.program_counter() as usize, value);
    }

    pub const fn negative(&self) -> bool {
        self.cpu.negative()
    }

    pub const fn zero(&self) -> bool {
        self.cpu.zero()
    }

    pub const fn carry(&self) -> bool {
        self.cpu.carry()
    }

    pub const fn overflow(&self) -> bool {
        self.cpu.overflow()
    }

    pub const fn program_counter(&self) -> u32 {
        self.cpu.program_counter()
    }

    pub const fn link_register(&self) -> u32 {
        self.cpu.link_register()
    }

    pub const fn stack_pointer(&self) -> u32 {
        self.cpu.stack_pointer()
    }

    pub fn rom(&self) -> u32 {
        self.bus.read_32(self.cpu.program_counter() as usize)
    }

    pub fn register(&self, register: u16) -> u32 {
        self.cpu.registers.get(register).expect("Invalid Register")
    }

    pub fn load_bios(&mut self) {
        if let Ok(file) = read("roms/bios.bin") {
            for (address, byte) in file.iter().enumerate() {
                self.bus.write_8(address, *byte);
            }
        } else {
            println!("Could not find BIOS");
        }
    }

    pub fn load_rom(&mut self, filename: &str) {
        if let Ok(file) = read(filename) {
            for (address, byte) in file.iter().enumerate() {
                self.bus
                    .write_8(address.wrapping_add(GAME_PAK_START_ADDRESS), *byte);
            }
        } else {
            println!("Could not read ROM: {filename}");
        }
    }

    pub fn framebuffer(&self) -> &[u16] {
        self.ppu.framebuffer()
    }
}
