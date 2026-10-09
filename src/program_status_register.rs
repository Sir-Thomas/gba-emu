#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CpuMode {
    #[default]
    Arm,
    Thumb,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Mode {
    #[default]
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    Undefined = 0b11011,
    System = 0b11111,
}

impl From<Mode> for u32 {
    fn from(val: Mode) -> Self {
        val as Self
    }
}

impl From<u32> for Mode {
    fn from(val: u32) -> Mode {
        match val & 0b11111 {
            0b10000 => Mode::User,
            0b10001 => Mode::Fiq,
            0b10010 => Mode::Irq,
            0b10011 => Mode::Supervisor,
            0b10111 => Mode::Abort,
            0b11011 => Mode::Undefined,
            0b11111 => Mode::System,
            _ => unreachable!("Invalid CPU Mode {val:#010X}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProgramStatusRegister {
    negative: bool,
    zero: bool,
    carry: bool,
    overflow: bool,
    irq_disable: bool,
    fiq_disable: bool,
    state: CpuMode,
    mode_bits: Mode,
}

impl From<ProgramStatusRegister> for u32 {
    fn from(val: ProgramStatusRegister) -> Self {
        let mut value = Self::from(val.mode_bits);
        if val.state == CpuMode::Thumb {
            value |= 0x0000_0020;
        }
        if val.fiq_disable {
            value |= 0x0000_0040;
        }
        if val.irq_disable {
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
            mode_bits: val.into(),
            state: if val & 0x0000_0020 > 0 {
                CpuMode::Thumb
            } else {
                CpuMode::Arm
            },
            fiq_disable: val & 0x0000_0040 > 0,
            irq_disable: val & 0x0000_0080 > 0,
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
pub struct SavedProgramStatusRegisters {
    current: ProgramStatusRegister,
    fiq: ProgramStatusRegister,
    irq: ProgramStatusRegister,
    supervisor: ProgramStatusRegister,
    abort: ProgramStatusRegister,
    undefined: ProgramStatusRegister,
}

impl SavedProgramStatusRegisters {
    pub fn set(&mut self, value: u32) {
        self.current.set(value);
    }

    pub fn set_fiq(&mut self, value: u32) {
        self.fiq.set(value);
    }

    pub fn set_irq(&mut self, value: u32) {
        self.irq.set(value);
    }

    pub fn set_supervisor(&mut self, value: u32) {
        self.supervisor.set(value);
    }

    pub fn set_abort(&mut self, value: u32) {
        self.abort.set(value);
    }

    pub fn set_undefined(&mut self, value: u32) {
        self.undefined.set(value);
    }

    pub fn current(&self) -> u32 {
        self.current.into()
    }

    pub fn fiq(&self) -> u32 {
        self.fiq.into()
    }

    pub fn irq(&self) -> u32 {
        self.irq.into()
    }

    pub fn supervisor(&self) -> u32 {
        self.supervisor.into()
    }

    pub fn abort(&self) -> u32 {
        self.abort.into()
    }

    pub fn undefined(&self) -> u32 {
        self.undefined.into()
    }

    pub const fn set_negative(&mut self, state: bool) {
        self.current.negative = state;
    }

    pub const fn negative(&self) -> bool {
        self.current.negative
    }

    pub const fn set_zero(&mut self, state: bool) {
        self.current.zero = state;
    }

    pub const fn zero(&self) -> bool {
        self.current.zero
    }

    pub const fn set_carry(&mut self, state: bool) {
        self.current.carry = state;
    }

    pub const fn carry(&self) -> bool {
        self.current.carry
    }

    pub const fn set_overflow(&mut self, state: bool) {
        self.current.overflow = state;
    }

    pub const fn overflow(&self) -> bool {
        self.current.overflow
    }

    pub const fn set_irq_disable(&mut self, state: bool) {
        self.current.irq_disable = state;
    }

    pub const fn irq_disable(&self) -> bool {
        self.current.irq_disable
    }

    pub const fn set_fiq_disable(&mut self, state: bool) {
        self.current.fiq_disable = state;
    }

    pub const fn fiq_disable(&self) -> bool {
        self.current.fiq_disable
    }

    pub const fn set_state(&mut self, state: CpuMode) {
        self.current.state = state;
    }

    pub const fn state(&self) -> CpuMode {
        self.current.state
    }

    pub const fn set_mode(&mut self, mode: Mode) {
        self.current.mode_bits = mode;
    }

    pub const fn mode(&self) -> Mode {
        self.current.mode_bits
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
            _ => unreachable!("Invalid conditions: {conditions:#04X}"),
        }
    }
}
