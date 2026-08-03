//! Risk metrics test suite entry point.
//!
//! This module consolidates tests for:
//!
//! - **convergence**: Analytical vs finite difference Greek convergence
//! - **determinism**: Deterministic results for identical inputs
//! - **edge_cases**: Boundary conditions and degenerate cases
//! - **graceful_metrics_test**: Graceful failure handling for metric computation
//! - **greek_relationships**: Mathematical relationships between Greeks
//! - **invariants**: Property-based tests for metric invariants
//! - **sign_conventions**: Correct sign conventions for all Greeks
//! - **vanna_volga_pockets**: Vanna-volga smile interpolation tests
//!
//! Run all metrics tests:
//! ```bash
//! cargo test --test metrics
//! ```

// Shared Test Utilities

/// Common test utilities: fixtures, tolerances, assertions, builders
#[path = "common/mod.rs"]
mod common;

#[path = "support/credit.rs"]
#[allow(dead_code, unused_imports)]
mod credit_support;
#[path = "support/date.rs"]
#[allow(dead_code, unused_imports)]
mod date_support;
#[path = "support/discount_forward_curves.rs"]
#[allow(dead_code, unused_imports)]
mod discount_forward_curve_support;
#[path = "support/equity_fx_options.rs"]
#[allow(dead_code, unused_imports)]
mod option_support;
#[path = "support/volatility.rs"]
#[allow(dead_code, unused_imports)]
mod volatility_support;

// Metrics Tests

#[path = "metrics/mod.rs"]
mod metrics;
