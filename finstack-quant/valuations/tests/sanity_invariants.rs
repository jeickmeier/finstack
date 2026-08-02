//! Sanity / invariant tests -- verify internal consistency of pricing.
//!
//! These tests do NOT compare against external references (QuantLib, Bloomberg).
//! They assert internal properties: par-rate self-consistency, pay/receive symmetry,
//! DV01 magnitude bands. For external-reference parity, see `tests/golden/`.

#[path = "support/credit.rs"]
#[allow(dead_code, unused_imports)]
mod credit_support;
#[path = "support/rates.rs"]
#[allow(dead_code, unused_imports)]
mod rates_support;

#[path = "sanity_invariants/mod.rs"]
mod sanity_invariants_tests;
