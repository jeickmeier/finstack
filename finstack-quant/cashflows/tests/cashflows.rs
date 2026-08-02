//! Cashflows test suite entry point for `finstack-quant-cashflows`.
//!
//! This module consolidates tests for:
//!
//! - **builder/**: Cashflow schedule generation, amortization, credit models (PSA/SDA)
//! - **schema_roundtrip**: JSON schema serialization roundtrip tests
//!
//! Run all cashflow tests:
//! ```bash
//! cargo test -p finstack-quant-cashflows --test cashflows
//! ```

/// Common test helpers: shared tolerances for rates, factors, and amounts.
#[path = "cashflows/helpers.rs"]
mod helpers;

/// Cashflow builder, schedule generation, amortization, and credit model tests
#[path = "cashflows/builder/mod.rs"]
mod builder;

/// JSON schema serialization roundtrip tests
#[path = "cashflows/schema_roundtrip.rs"]
mod schema_roundtrip;
