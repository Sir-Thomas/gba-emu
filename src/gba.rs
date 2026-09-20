#![allow(dead_code)]

use crate::bus::BUS;
use crate::cpu::CPU;

const DISPLAY_WIDTH: usize = 240;
const DISPLAY_HEIGHT: usize = 160;

pub struct GBA {
    cpu: CPU,
    bus: BUS,
}
