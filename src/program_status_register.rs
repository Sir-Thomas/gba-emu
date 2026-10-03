#[derive(Default)]
pub struct ProgramStatusRegister {
    negative: bool,
    zero: bool,
    carry: bool,
    overflow: bool,
    irq_disable: bool,
    fiq_disable: bool,
    state_bit: bool,
    mode_bits: u8,
}

impl From<ProgramStatusRegister> for u32 {
    fn from(val: ProgramStatusRegister) -> Self {
        let mut value = u32::from(val.mode_bits);
        if val.state_bit {
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
    pub fn set_negative(&mut self, state: bool) {
        self.negative = state;
    }

    pub fn get_negative(&self) -> bool {
        self.negative
    }

    pub fn set_zero(&mut self, state: bool) {
        self.zero = state;
    }

    pub fn get_zero(&self) -> bool {
        self.zero
    }

    pub fn set_carry(&mut self, state: bool) {
        self.carry = state;
    }

    pub fn get_carry(&self) -> bool {
        self.carry
    }

    pub fn set_overflow(&mut self, state: bool) {
        self.overflow = state;
    }

    pub fn get_overflow(&self) -> bool {
        self.overflow
    }

    pub fn set_irq_disable(&mut self, state: bool) {
        self.irq_disable = state;
    }

    pub fn get_irq_disable(&self) -> bool {
        self.irq_disable
    }

    pub fn set_fiq_disable(&mut self, state: bool) {
        self.fiq_disable = state;
    }

    pub fn get_fiq_disable(&self) -> bool {
        self.fiq_disable
    }

    pub fn set_state_bit(&mut self, state: bool) {
        self.state_bit = state;
    }

    pub fn get_state_bit(&self) -> bool {
        self.state_bit
    }
}
