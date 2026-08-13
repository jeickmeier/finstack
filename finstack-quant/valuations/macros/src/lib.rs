#![forbid(unsafe_code)]
#![warn(clippy::float_cmp)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        clippy::float_cmp,
    )
)]
// Allow expect() in doc tests (they are test code)
#![doc(test(attr(allow(clippy::expect_used))))]

//! Procedural macros for the finstack-quant-valuations crate.
//!
//! This crate is not umbrella-re-exported and is not intended for direct
//! application use. It currently provides:
//!
//! - `FinancialBuilder`: generates type-safe builder patterns for instrument
//!   structs in `finstack-quant-valuations`

use proc_macro::TokenStream;

mod financial_builder;

/// Derives a builder pattern for financial instrument structs.
///
/// See the `financial_builder` module for detailed documentation.
#[proc_macro_derive(FinancialBuilder, attributes(builder))]
pub fn derive_financial_builder(input: TokenStream) -> TokenStream {
    financial_builder::derive_financial_builder_impl(input)
}
