#![allow(dead_code)]

const BIOS_ROM_SIZE: usize = 0x4000;
const EXTERNAL_WORKING_RAM_SIZE: usize = 0x0004_0000;
const INTERNAL_WORKING_RAM_SIZE: usize = 0x8000;
const IO_REGISTERS_SIZE: usize = 0x03FF;
const PALETTE_RAM_SIZE: usize = 0x0400;
const VRAM_SIZE: usize = 0x0024_0000;
const OAM_SIZE: usize = 0x0400;

const BIOS_ROM_START_ADDRESS: usize = 0x0000_0000;
const EXTERNAL_WORKING_RAM_START_ADDRESS: usize = 0x0200_0000;
const INTERNAL_WORKING_RAM_START_ADDRESS: usize = 0x0300_0000;
const IO_REGISTERS_START_ADDRESS: usize = 0x0400_0000;
const PALETTE_RAM_START_ADDRESS: usize = 0x0500_0000;
const VRAM_START_ADDRESS: usize = 0x0600_0000;
const OAM_START_ADDRESS: usize = 0x0700_0000;

const BIOS_ROM_END_ADDRESS: usize = BIOS_ROM_START_ADDRESS + BIOS_ROM_SIZE;
const EXTERNAL_WORKING_RAM_END_ADDRESS: usize =
    EXTERNAL_WORKING_RAM_START_ADDRESS + EXTERNAL_WORKING_RAM_SIZE;
const INTERNAL_WORKING_RAM_END_ADDRESS: usize =
    INTERNAL_WORKING_RAM_START_ADDRESS + INTERNAL_WORKING_RAM_SIZE;
const IO_REGISTERS_END_ADDRESS: usize = IO_REGISTERS_START_ADDRESS + IO_REGISTERS_SIZE;
const PALETTE_RAM_END_ADDRESS: usize = PALETTE_RAM_START_ADDRESS + PALETTE_RAM_SIZE;
const VRAM_END_ADDRESS: usize = VRAM_START_ADDRESS + VRAM_SIZE;
const OAM_END_ADDRESS: usize = OAM_START_ADDRESS + OAM_SIZE;

pub struct BUS {
    bios_rom: [u8; BIOS_ROM_SIZE],
    external_working_ram: [u8; EXTERNAL_WORKING_RAM_SIZE],
    internal_working_ram: [u8; INTERNAL_WORKING_RAM_SIZE],
    io_registers: [u8; IO_REGISTERS_SIZE],
    palette_ram: [u8; PALETTE_RAM_SIZE],
    vram: [u8; VRAM_SIZE],
    oam: [u8; OAM_SIZE],
}

impl BUS {
    fn new() -> Self {
        Self {
            bios_rom: [0; BIOS_ROM_SIZE],
            external_working_ram: [0; EXTERNAL_WORKING_RAM_SIZE],
            internal_working_ram: [0; INTERNAL_WORKING_RAM_SIZE],
            io_registers: [0; IO_REGISTERS_SIZE],
            palette_ram: [0; PALETTE_RAM_SIZE],
            vram: [0; VRAM_SIZE],
            oam: [0; OAM_SIZE],
        }
    }

    pub fn get(&self, address: usize) {
        match address {
            // General Internal Memory
            BIOS_ROM_START_ADDRESS..BIOS_ROM_END_ADDRESS => self.get_bios(address),
            EXTERNAL_WORKING_RAM_START_ADDRESS..EXTERNAL_WORKING_RAM_END_ADDRESS => {
                self.get_external_working_ram(address)
            }
            INTERNAL_WORKING_RAM_START_ADDRESS..INTERNAL_WORKING_RAM_END_ADDRESS => {
                self.get_internal_working_ram(address)
            }
            IO_REGISTERS_START_ADDRESS..IO_REGISTERS_END_ADDRESS => self.get_io(address),
            PALETTE_RAM_START_ADDRESS..PALETTE_RAM_END_ADDRESS => self.get_palette_ram(address),
            VRAM_START_ADDRESS..VRAM_END_ADDRESS => self.get_vram(address),
            OAM_START_ADDRESS..OAM_END_ADDRESS => self.get_oam(address),
            _ => {}
        }
    }

    fn get_bios(&self, address: usize) {
        let index = address.saturating_sub(BIOS_ROM_START_ADDRESS);
        let data = self.bios_rom.get(index);
        todo!();
    }

    fn get_external_working_ram(&self, address: usize) {
        let index = address.saturating_sub(EXTERNAL_WORKING_RAM_START_ADDRESS);
        let data = self.external_working_ram.get(index);
        todo!();
    }

    fn get_internal_working_ram(&self, address: usize) {
        let index = address.saturating_sub(INTERNAL_WORKING_RAM_START_ADDRESS);
        let data = self.internal_working_ram.get(index);
        todo!();
    }

    fn get_io(&self, address: usize) {
        let index = address.saturating_sub(IO_REGISTERS_START_ADDRESS);
        let data = self.io_registers.get(index);
        todo!();
    }

    fn get_palette_ram(&self, address: usize) {
        let index = address.saturating_sub(PALETTE_RAM_START_ADDRESS);
        let data = self.palette_ram.get(index);
        todo!();
    }

    fn get_vram(&self, address: usize) {
        let index = address.saturating_sub(VRAM_START_ADDRESS);
        let data = self.vram.get(index);
        todo!();
    }

    fn get_oam(&self, address: usize) {
        let index = address.saturating_sub(OAM_START_ADDRESS);
        let data = self.oam.get(index);
        todo!();
    }
}
