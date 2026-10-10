trait Bit {
    fn bit(&self, bit: usize) -> bool;
}

impl Bit for u32 {
    fn bit(&self, bit: usize) -> bool {
        self & (1 << (bit % 32)) > 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AluResult {
    pub result: u32,
    pub carry: Option<bool>,
    pub overflow: Option<bool>,
}

pub fn shift_left(x: u32, shift: u32) -> AluResult {
    match shift {
        0 => AluResult {
            result: x,
            carry: None,
            overflow: None,
        },
        1..=31 => AluResult {
            result: x << shift,
            carry: Some(x.bit(32usize.wrapping_sub(shift as usize))),
            overflow: None,
        },
        32 => AluResult {
            result: 0,
            carry: Some(x.bit(0)),
            overflow: None,
        },
        _ => AluResult {
            result: 0,
            carry: Some(false),
            overflow: None,
        },
    }
}

pub fn shift_right(x: u32, shift: u32) -> AluResult {
    match shift {
        0 => AluResult {
            result: x,
            carry: None,
            overflow: None,
        },
        1..=31 => AluResult {
            result: x >> shift,
            carry: Some(x.bit((shift as usize).wrapping_sub(1))),
            overflow: None,
        },
        32 => AluResult {
            result: 0,
            carry: Some(x.bit(31)),
            overflow: None,
        },
        _ => AluResult {
            result: 0,
            carry: Some(false),
            overflow: None,
        },
    }
}

pub fn arithmetic_shift_right(x: u32, shift: u32) -> AluResult {
    match shift {
        0 => AluResult {
            result: x,
            carry: None,
            overflow: None,
        },
        1..=31 => AluResult {
            result: (x.cast_signed() >> shift).cast_unsigned(),
            carry: Some(x.bit((shift as usize).wrapping_sub(1))),
            overflow: None,
        },
        _ => {
            let bit = x.bit(31);
            let value = if bit { 0xFFFF_FFFF } else { 0x0000_0000 };
            AluResult {
                result: value,
                carry: Some(bit),
                overflow: None,
            }
        }
    }
}

pub fn rotate_right(x: u32, shift: u32) -> AluResult {
    if shift == 0 {
        return AluResult {
            result: x,
            carry: None,
            overflow: None,
        };
    }
    let s = shift as usize % 32;
    let carry = x.bit(if s == 0 { 31 } else { s.wrapping_sub(1) });
    AluResult {
        result: x.rotate_right(shift),
        carry: Some(carry),
        overflow: None,
    }
}

pub fn rotate_right_carry(value: u32, c: bool) -> AluResult {
    let alu_result = shift_right(value, 1);
    let result = alu_result.result | u32::from(c) << 31;
    AluResult {
        result,
        carry: alu_result.carry,
        overflow: None,
    }
}

pub fn nop(x: u32) -> AluResult {
    AluResult {
        result: x,
        carry: None,
        overflow: None,
    }
}

pub fn and(x: u32, y: u32) -> AluResult {
    AluResult {
        result: x & y,
        carry: None,
        overflow: None,
    }
}

pub fn xor(x: u32, y: u32) -> AluResult {
    AluResult {
        result: x ^ y,
        carry: None,
        overflow: None,
    }
}

pub fn or(x: u32, y: u32) -> AluResult {
    AluResult {
        result: x | y,
        carry: None,
        overflow: None,
    }
}

pub fn add(x: u32, y: u32) -> AluResult {
    let (result, carry) = x.overflowing_add(y);
    let (_, overflow) = x.cast_signed().overflowing_add(y.cast_signed());
    AluResult {
        result,
        carry: Some(carry),
        overflow: Some(overflow),
    }
}

pub fn subtract(x: u32, y: u32) -> AluResult {
    let (result, borrow) = x.overflowing_sub(y);
    let (_, overflow) = x.cast_signed().overflowing_sub(y.cast_signed());
    AluResult {
        result,
        carry: Some(!borrow),
        overflow: Some(overflow),
    }
}

pub fn add_carry(x: u32, y: u32, carry: u32) -> AluResult {
    let (temp, c1) = x.overflowing_add(y);
    let (result, c2) = temp.overflowing_add(carry);
    let (temp, o1) = x.cast_signed().overflowing_add(y.cast_signed());
    let (_, o2) = temp.overflowing_add(carry.cast_signed());
    AluResult {
        result,
        carry: Some(c1 | c2),
        overflow: Some(o1 | o2),
    }
}

pub fn sub_carry(x: u32, y: u32, carry: u32) -> AluResult {
    let (temp, b1) = x.overflowing_sub(y);
    let (result, b2) = temp.wrapping_add(carry).overflowing_sub(1);
    let (temp, o1) = x.cast_signed().overflowing_sub(y.cast_signed());
    let (_, o2) = temp.wrapping_add(carry.cast_signed()).overflowing_sub(1);
    AluResult {
        result,
        carry: Some(!(b1 | b2)),
        overflow: Some(o1 | o2),
    }
}
