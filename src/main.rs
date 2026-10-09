#![feature(integer_widen_truncate)]

mod app;
mod bus;
mod cpu;
mod gba;
mod ppu;
mod program_status_register;

use crate::gba::Gba;
use eframe::{NativeOptions, Result, run_native};

fn main() -> Result {
    let options = NativeOptions::default();

    let mut args = std::env::args();
    args.next();
    let rom: String = args
        .next()
        .unwrap_or_else(|| String::from("roms/thumb.gba"));

    let mut gba = Gba::new();
    gba.load_bios();
    gba.load_rom(&rom);

    run_native(
        "GBA",
        options,
        Box::new(|cc| Ok(Box::new(app::GbaApp::new(cc, gba)))),
    )
}
