#![allow(dead_code)]

use crate::bus::Bus;
use crate::cpu::{Cpu, ThumbInstruction};

pub const DISPLAY_WIDTH: usize = 240;
pub const DISPLAY_HEIGHT: usize = 160;

pub struct Gba {
    cpu: Cpu,
    bus: Bus,
}

impl Gba {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::default(),
            bus: Bus::new(),
        }
    }

    pub fn cpu_cycle(&mut self) {
        self.cpu.cpu_cycle(&mut self.bus);
    }

    pub fn get_next_instruction(&self) -> (u16, ThumbInstruction) {
        self.cpu.get_next_instruction(&self.bus)
    }

    pub fn insert_opcode(&mut self, value: u16) {
        self.bus.write_16(self.cpu.get_program_counter() as usize, value);
    }

    pub fn get_program_counter(&self) -> u32 {
        self.cpu.get_program_counter()
    }

    pub fn get_rom(&self) -> u32 {
        self.bus.read_32(self.cpu.get_program_counter() as usize)
    }

    //temp
    pub fn get_r00(&self) -> u32 {
        self.cpu.get_r00()
    }

    //temp
    pub fn get_r01(&self) -> u32 {
        self.cpu.get_r01()
    }

    //temp
    pub fn set_r00(&mut self, value: u32) {
        self.cpu.set_r00(value);
    }
}
