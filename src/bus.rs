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

pub struct Bus {
    // TODO: Switch these back to Boxed arrays (do they have to be boxed?)
    // Vec allows me to get rid of large stack allocations with 0 effort, but these should be arrays
    // so that they have a fixed size that the compiler can guaruntee for me. I will just have to
    // come up with a good way to initialize them without using the stack.
    bios_rom: Vec<u8>,
    external_working_ram: Vec<u8>,
    internal_working_ram: Vec<u8>,
    io_registers: Vec<u8>,
    palette_ram: Vec<u8>,
    vram: Vec<u8>,
    oam: Vec<u8>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            bios_rom: vec![0; BIOS_ROM_SIZE],
            external_working_ram: vec![0; EXTERNAL_WORKING_RAM_SIZE],
            internal_working_ram: vec![0; INTERNAL_WORKING_RAM_SIZE],
            io_registers: vec![0; IO_REGISTERS_SIZE],
            palette_ram: vec![0; PALETTE_RAM_SIZE],
            vram: vec![0; VRAM_SIZE],
            oam: vec![0; OAM_SIZE],
        }
    }

    pub fn write_16(&mut self, address: usize, value: u16) {
        self.bios_rom[address] = ((value & 0xFF00) >> 8) as u8;
        self.bios_rom[address + 1] = (value & 0xFF) as u8;
    }

    pub fn read_8(&self, address: usize) -> u16 {
        let data = self.read_32(address);
        let data8 = (data & 0xFF00_0000) >> 24;
        data8.truncate()
    }

    pub fn read_16(&self, address: usize) -> u16 {
        let data = self.read_32(address);
        let data16 = (data & 0xFFFF_0000) >> 16;
        data16.truncate()
    }

    pub fn read_32(&self, address: usize) -> u32 {
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
            _ => 0x0000_0000
        }
    }

    fn get_bios(&self, address: usize) -> u32 {
        let index = address.saturating_sub(BIOS_ROM_START_ADDRESS);
        let data = self.bios_rom.get(index..index+4).unwrap();
        ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
    }

    fn get_external_working_ram(&self, address: usize) -> u32 {
        let index = address.saturating_sub(EXTERNAL_WORKING_RAM_START_ADDRESS);
        let data = self.external_working_ram.get(index..index+4).unwrap();
        ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
    }

    fn get_internal_working_ram(&self, address: usize) -> u32 {
        let index = address.saturating_sub(INTERNAL_WORKING_RAM_START_ADDRESS);
        let data = self.internal_working_ram.get(index..index+4).unwrap();
        ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
    }

    fn get_io(&self, address: usize) -> u32 {
        let index = address.saturating_sub(IO_REGISTERS_START_ADDRESS);
        let data = self.io_registers.get(index..index+4).unwrap();
        ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
    }

    fn get_palette_ram(&self, address: usize) -> u32 {
        let index = address.saturating_sub(PALETTE_RAM_START_ADDRESS);
        let data = self.palette_ram.get(index..index+4).unwrap();
        ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
    }

    fn get_vram(&self, address: usize) -> u32 {
        let index = address.saturating_sub(VRAM_START_ADDRESS);
        let data = self.vram.get(index..index+4).unwrap();
        ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
    }

    fn get_oam(&self, address: usize) -> u32 {
        let index = address.saturating_sub(OAM_START_ADDRESS);
        let data = self.oam.get(index..index+4).unwrap();
        ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
    }
}
