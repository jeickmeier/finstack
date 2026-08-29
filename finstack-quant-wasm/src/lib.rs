//! WebAssembly bindings for the Finstack Quant financial computation library.
//!
//! The public API is consumed through a hand-written JS/TS facade (`index.js`)
//! that groups raw `wasm-bindgen` exports into crate-level namespaces mirroring
//! the Rust umbrella crate structure.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::float_cmp,
    )
)]

use wasm_bindgen::prelude::*;

pub mod api;
pub mod utils;

#[wasm_bindgen(start)]
/// Module initializer: installs the panic hook when `console_panic_hook` is enabled.
pub fn start() {
    #[cfg(feature = "console_panic_hook")]
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_does_not_panic() {
        start();
    }
}
