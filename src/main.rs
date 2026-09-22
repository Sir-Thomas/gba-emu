#![feature(integer_widen_truncate)]

mod app;
mod bus;
mod cpu;
mod gba;
mod program_status_register;

use crate::gba::Gba;
use eframe::{NativeOptions, Result, run_native};

fn main() -> Result {
    let options = NativeOptions::default();

    let gba = Gba::new();

    run_native(
        "GBA",
        options,
        Box::new(|cc| {
            Ok(Box::new(app::GbaApp::new(cc, gba)))
        }),
    )
}
