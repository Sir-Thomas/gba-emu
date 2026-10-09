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
        self.cpu.register(register)
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
        self.ppu.draw(&mut self.bus)
    }
}
