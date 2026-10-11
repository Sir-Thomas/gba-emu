#![allow(dead_code)]

use crate::cpu::AluResult;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InstructionSet {
    #[default]
    Arm,
    Thumb,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CpuMode {
    #[default]
    User,
    Fiq,
    Irq,
    Supervisor,
    Abort,
    Undefined,
    System,
}

impl From<CpuMode> for u32 {
    fn from(val: CpuMode) -> Self {
        match val {
            CpuMode::User => 0b10000,
            CpuMode::Fiq => 0b10001,
            CpuMode::Irq => 0b10010,
            CpuMode::Supervisor => 0b10011,
            CpuMode::Abort => 0b10111,
            CpuMode::Undefined => 0b11011,
            CpuMode::System => 0b11111,
        }
    }
}

impl From<u32> for CpuMode {
    fn from(val: u32) -> CpuMode {
        match val & 0b11111 {
            0b10000 => CpuMode::User,
            0b10001 => CpuMode::Fiq,
            0b10010 => CpuMode::Irq,
            0b10011 => CpuMode::Supervisor,
            0b10111 => CpuMode::Abort,
            0b11011 => CpuMode::Undefined,
            0b11111 => CpuMode::System,
            _ => panic!("Invalid CPU Mode {val:#010X}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProgramStatusRegister {
    pub negative: bool,
    pub zero: bool,
    pub carry: bool,
    pub overflow: bool,
    pub irq_disabled: bool,
    pub fiq_disabled: bool,
    pub instruction_set: InstructionSet,
    pub cpu_mode: CpuMode,
}

impl From<ProgramStatusRegister> for u32 {
    fn from(val: ProgramStatusRegister) -> Self {
        let mut value = Self::from(val.cpu_mode);
        if val.instruction_set == InstructionSet::Thumb {
            value |= 0x0000_0020;
        }
        if val.fiq_disabled {
            value |= 0x0000_0040;
        }
        if val.irq_disabled {
            value |= 0x0000_0080;
        }
        if val.overflow {
            value |= 0x1000_0000;
        }
        if val.carry {
            value |= 0x2000_0000;
        }
        if val.zero {
            value |= 0x4000_0000;
        }
        if val.negative {
            value |= 0x8000_0000;
        }
        value
    }
}

impl From<u32> for ProgramStatusRegister {
    fn from(val: u32) -> Self {
        Self {
            cpu_mode: val.into(),
            instruction_set: if val & 0x0000_0020 > 0 {
                InstructionSet::Thumb
            } else {
                InstructionSet::Arm
            },
            fiq_disabled: val & 0x0000_0040 > 0,
            irq_disabled: val & 0x0000_0080 > 0,
            overflow: val & 0x1000_0000 > 0,
            carry: val & 0x2000_0000 > 0,
            zero: val & 0x4000_0000 > 0,
            negative: val & 0x8000_0000 > 0,
        }
    }
}

impl ProgramStatusRegister {
    pub fn set(&mut self, value: u32) {
        *self = value.into();
    }
}

#[derive(Default)]
pub struct StatusRegisters {
    pub current: ProgramStatusRegister,
    saved_fiq: ProgramStatusRegister,
    saved_irq: ProgramStatusRegister,
    saved_supervisor: ProgramStatusRegister,
    saved_abort: ProgramStatusRegister,
    saved_undefined: ProgramStatusRegister,
}

impl StatusRegisters {
    pub fn set_current(&mut self, value: u32) {
        self.current.set(value);
    }

    pub fn save(&mut self, value: u32) {
        match self.current.cpu_mode {
            CpuMode::Fiq => self.saved_fiq.set(value),
            CpuMode::Irq => self.saved_irq.set(value),
            CpuMode::Supervisor => self.saved_supervisor.set(value),
            CpuMode::Abort => self.saved_abort.set(value),
            CpuMode::Undefined => self.saved_undefined.set(value),
            mode => panic!("Cannot save PSR for mode: {mode:?}"),
        }
    }

    pub fn load(&mut self) {
        self.current.set(self.get_saved());
    }

    pub fn set_supervisor(&mut self, value: u32) {
        self.saved_supervisor.set(value);
    }

    pub fn get_saved(&self) -> u32 {
        match self.current.cpu_mode {
            CpuMode::Fiq => self.saved_fiq.into(),
            CpuMode::Irq => self.saved_irq.into(),
            CpuMode::Supervisor => self.saved_supervisor.into(),
            CpuMode::Abort => self.saved_abort.into(),
            CpuMode::Undefined => self.saved_undefined.into(),
            mode => panic!("Cannot get saved PSR for mode: {mode:?}"),
        }
    }

    pub fn set_conditions(&mut self, alu_result: AluResult) {
        self.current.negative = alu_result.result & 0x8000_0000 > 0;
        self.current.zero = alu_result.result == 0;
        if let Some(c) = alu_result.carry {
            self.current.carry = c;
        }
        if let Some(v) = alu_result.overflow {
            self.current.overflow = v;
        }
    }

    pub fn set_carry(&mut self, carry: Option<bool>) {
        if let Some(c) = carry {
            self.current.carry = c;
        }
    }

    pub fn check_conditions(&self, conditions: u8) -> bool {
        match conditions {
            0b0000 => self.current.zero,
            0b0001 => !self.current.zero,
            0b0010 => self.current.carry,
            0b0011 => !self.current.carry,
            0b0100 => self.current.negative,
            0b0101 => !self.current.negative,
            0b0110 => self.current.overflow,
            0b0111 => !self.current.overflow,
            0b1000 => self.current.carry && !self.current.zero,
            0b1001 => !self.current.carry || self.current.zero,
            0b1010 => self.current.negative == self.current.overflow,
            0b1011 => self.current.negative != self.current.overflow,
            0b1100 => !self.current.zero && (self.current.negative == self.current.overflow),
            0b1101 => self.current.zero || (self.current.negative != self.current.overflow),
            0b1110 => true,
            _ => panic!("Invalid conditions: {conditions:#04X}"),
        }
    }
}

#[derive(Default)]
pub struct Registers {
    registers: [u32; 13],
    fiq_registers: [u32; 5],
    pub stack_pointer: u32,
    pub link_register: u32,
    pub program_counter: u32,
    pub status_registers: StatusRegisters,
}

impl Registers {
    pub fn set_stack_pointer(&mut self, value: u32) {
        self.stack_pointer = value;
    }

    pub fn set_program_counter(&mut self, value: u32) {
        self.program_counter = value;
    }

    pub fn set_instruction_set(&mut self, instruction_set: InstructionSet) {
        self.status_registers.current.instruction_set = instruction_set;
    }

    pub fn get(&self, register: u16) -> u32 {
        match register {
            0..=7 => self.registers[register as usize],
            8..=12 => {
                if self.status_registers.current.cpu_mode == CpuMode::Fiq {
                    self.fiq_registers[register as usize - 8]
                } else {
                    self.registers[register as usize]
                }
            }
            13 => self.stack_pointer,
            14 => self.link_register,
            15 => {
                if self.status_registers.current.instruction_set == InstructionSet::Arm {
                    self.program_counter.wrapping_add(4)
                } else {
                    self.program_counter.wrapping_add(2)
                }
            }
            _ => panic!("Invalid Register: {register}"),
        }
    }

    pub fn set(&mut self, register: u16, value: u32) {
        match register {
            0..=7 => self.registers[register as usize] = value,
            8..=12 => {
                if self.status_registers.current.cpu_mode == CpuMode::Fiq {
                    self.fiq_registers[register as usize - 8] = value
                } else {
                    self.registers[register as usize] = value
                }
            }
            13 => self.stack_pointer = value,
            14 => self.link_register = value,
            15 => self.program_counter = value,
            _ => panic!("Invalid Register: {register}"),
        }
    }

    pub fn current(&self) -> u32 {
        self.status_registers.current.into()
    }

    pub const fn set_negative(&mut self, state: bool) {
        self.status_registers.current.negative = state;
    }

    pub const fn negative(&self) -> bool {
        self.status_registers.current.negative
    }

    pub const fn set_zero(&mut self, state: bool) {
        self.status_registers.current.zero = state;
    }

    pub const fn zero(&self) -> bool {
        self.status_registers.current.zero
    }

    pub const fn set_carry(&mut self, state: bool) {
        self.status_registers.current.carry = state;
    }

    pub const fn carry(&self) -> bool {
        self.status_registers.current.carry
    }

    pub const fn set_overflow(&mut self, state: bool) {
        self.status_registers.current.overflow = state;
    }

    pub const fn overflow(&self) -> bool {
        self.status_registers.current.overflow
    }

    pub const fn set_irq_disabled(&mut self, state: bool) {
        self.status_registers.current.irq_disabled = state;
    }

    pub const fn irq_disabled(&self) -> bool {
        self.status_registers.current.irq_disabled
    }

    pub const fn set_fiq_disabled(&mut self, state: bool) {
        self.status_registers.current.fiq_disabled = state;
    }

    pub const fn fiq_disabled(&self) -> bool {
        self.status_registers.current.fiq_disabled
    }

    pub const fn set_state(&mut self, state: InstructionSet) {
        self.status_registers.current.instruction_set = state;
    }

    pub const fn state(&self) -> InstructionSet {
        self.status_registers.current.instruction_set
    }

    pub const fn set_mode(&mut self, mode: CpuMode) {
        self.status_registers.current.cpu_mode = mode;
    }

    pub const fn mode(&self) -> CpuMode {
        self.status_registers.current.cpu_mode
    }

    pub fn check_conditions(&self, conditions: u8) -> bool {
        self.status_registers.check_conditions(conditions)
    }
}
