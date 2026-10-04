#![allow(dead_code)]

use std::fs::read;

use crate::bus::{Bus, GAME_PAK_START_ADDRESS};
use crate::cpu::{Cpu, Instruction};
use crate::ppu::Ppu;

pub const DISPLAY_WIDTH: usize = 240;
pub const DISPLAY_HEIGHT: usize = 160;

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

    pub fn cpu_cycle(&mut self) {
        self.cpu.cpu_cycle(&mut self.bus);
    }

    pub fn next_instruction(&self) -> (u32, Instruction) {
        self.cpu.next_instruction(&self.bus)
    }

    pub fn insert_opcode(&mut self, value: u16) {
        self.bus
            .write_16(self.cpu.program_counter() as usize, value);
    }

    pub const fn program_counter(&self) -> u32 {
        self.cpu.program_counter()
    }

    pub fn rom(&self) -> u32 {
        self.bus.read_32(self.cpu.program_counter() as usize)
    }

    //temp
    pub const fn r00(&self) -> u32 {
        self.cpu.r00()
    }

    //temp
    pub const fn r01(&self) -> u32 {
        self.cpu.r01()
    }

    //temp
    pub const fn set_r00(&mut self, value: u32) {
        self.cpu.set_r00(value);
    }

    pub fn load_rom(&mut self) {
        let filename = "roms/suite.gba";
        if let Ok(file) = read(filename) {
            for (address, byte) in file.iter().enumerate() {
                self.bus
                    .write_8(address.wrapping_add(GAME_PAK_START_ADDRESS), *byte);
            }
        } else {
            println!("Could not read ROM: {filename}");
        }
    }

    pub fn draw(&mut self) -> &[u16] {
        self.ppu.draw(&self.bus)
    }
}
