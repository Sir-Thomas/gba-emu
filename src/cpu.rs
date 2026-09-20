#![allow(dead_code)]

pub struct CPU {
    r00: u32,
    r01: u32,
    r02: u32,
    r03: u32,
    r04: u32,
    r05: u32,
    r06: u32,
    r07: u32,
    r08: u32,
    r09: u32,
    r10: u32,
    r11: u32,
    r12: u32,
    stack_pointer: u32,
    link_register: u32,
    program_counter: u32,
}
