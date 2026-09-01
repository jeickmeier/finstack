//! Build script for finstack-quant-core: generates calendar implementations from JSON.

#[path = "build/generate_calendars.rs"]
mod generate_calendars;
#[path = "build/generate_sifma_settlements.rs"]
mod generate_sifma_settlements;

use std::io;

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=data/sifma_settlements.csv");
    println!("cargo:rerun-if-changed=data/calendars");
    println!("cargo:rerun-if-changed=build/generate_calendars.rs");
    println!("cargo:rerun-if-changed=build/generate_sifma_settlements.rs");
    generate_calendars::generate()?;
    generate_sifma_settlements::generate()
}
