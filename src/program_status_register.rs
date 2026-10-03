#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CpuMode {
    Arm,
    #[default]
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
            value |= 0x20;
        }
        if val.fiq_disable {
            value |= 0x40;
        }
        if val.irq_disable {
            value |= 0x80;
        }
        if val.overflow {
            value |= 0x1000;
        }
        if val.carry {
            value |= 0x2000;
        }
        if val.zero {
            value |= 0x4000;
        }
        if val.negative {
            value |= 0x8000;
        }
        value
    }
}

impl ProgramStatusRegister {
    pub const fn set_negative(&mut self, state: bool) {
        self.negative = state;
    }

    pub const fn negative(self) -> bool {
        self.negative
    }

    pub const fn set_zero(&mut self, state: bool) {
        self.zero = state;
    }

    pub const fn zero(self) -> bool {
        self.zero
    }

    pub const fn set_carry(&mut self, state: bool) {
        self.carry = state;
    }

    pub const fn carry(self) -> bool {
        self.carry
    }

    pub const fn set_overflow(&mut self, state: bool) {
        self.overflow = state;
    }

    pub const fn overflow(self) -> bool {
        self.overflow
    }

    pub const fn set_irq_disable(&mut self, state: bool) {
        self.irq_disable = state;
    }

    pub const fn irq_disable(self) -> bool {
        self.irq_disable
    }

    pub const fn set_fiq_disable(&mut self, state: bool) {
        self.fiq_disable = state;
    }

    pub const fn fiq_disable(self) -> bool {
        self.fiq_disable
    }

    pub const fn set_state(&mut self, state: CpuMode) {
        self.state = state;
    }

    pub const fn state(self) -> CpuMode {
        self.state
    }

    pub const fn set_mode(&mut self, mode: Mode) {
        self.mode_bits = mode;
    }

    pub const fn mode(self) -> Mode {
        self.mode_bits
    }
}
